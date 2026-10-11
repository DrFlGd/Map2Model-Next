//! Deterministic 3MF Core 1.4.0 single-mesh OPC writer with independent
//! ZIP/XML readback. Slicer acceptance and the official Core test suite
//! remain separate external release gates.
use crate::{prepare_export, ExportError, PreparedExport, MODEL_PART, PROVENANCE_PART};
use map2model_contracts::canonical::{canonical_mesh_digest, MeshTriangle};
use map2model_contracts::{ElevationGridV1, TerrainBuildRequestV1, TerrainSourceManifestV1};
use map2model_mesh::{BuildOutput, TerrainMesh};
use roxmltree::Document;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const CORE_NS: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const OPC_RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const OPC_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const START_REL: &str = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";
const PRESERVE_REL: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/mustpreserve";
const MODEL_TYPE: &str = "application/vnd.ms-package.3dmanufacturing-3dmodel+xml";
const MAX_PART_SIZE: usize = 64 * 1024 * 1024;
const MAX_PACKAGE_SIZE: usize = 80 * 1024 * 1024;
const CONTENT_TYPES: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
    "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">",
    "<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>",
    "<Override PartName=\"/3D/3dmodel.model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>",
    "<Override PartName=\"/Metadata/map2model.provenance.json\" ContentType=\"application/json\"/>",
    "</Types>"
);
const RELATIONSHIPS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
    "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    "<Relationship Id=\"rel-model\" Target=\"/3D/3dmodel.model\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>",
    "<Relationship Id=\"rel-provenance\" Target=\"/Metadata/map2model.provenance.json\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/mustpreserve\"/>",
    "</Relationships>"
);

/// Actual ZIP bytes plus separately verified file/provenance/geometry SHA-256.
/// This does NOT assert official Core-suite or third-party slicer acceptance.
#[derive(Debug, Clone)]
pub struct Exported3mf {
    pub bytes: Vec<u8>,
    pub package_sha256: String,
    pub provenance_sha256: String,
    pub canonical_geometry_sha256: String,
    pub vertices: usize,
    pub triangles: usize,
    pub aabb_mm: [f64; 6],
}

#[derive(Debug)]
pub struct Inspected3mf {
    pub package_sha256: String,
    pub provenance_sha256: String,
    pub canonical_geometry_sha256: String,
    pub vertices: usize,
    pub triangles: usize,
    pub aabb_mm: [f64; 6],
}

fn invalid(message: impl Into<String>) -> ExportError {
    ExportError::new("INVALID_EXPORT", message)
}
fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            c if c.is_control() && c != '\n' && c != '\t' && c != '\r' => {
                escaped.push(' ');
            }
            c => escaped.push(c),
        }
    }
    escaped
}
fn metadata(xml: &mut String, name: &str, value: &str) {
    xml.push_str("<metadata name=\"m2m:");
    xml.push_str(name);
    xml.push_str("\" preserve=\"true\">");
    xml.push_str(&xml_escape(value));
    xml.push_str("</metadata>");
}
fn model_xml(
    mesh: &TerrainMesh,
    prepared: &PreparedExport,
    source: &TerrainSourceManifestV1,
    request: &TerrainBuildRequestV1,
) -> Result<String, ExportError> {
    let mut xml =
        String::with_capacity(mesh.vertices_mm.len() * 105 + mesh.triangles.len() * 65 + 1400);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml.push_str("<model xmlns=\"");
    xml.push_str(CORE_NS);
    xml.push_str("\" xmlns:m2m=\"https://map2model.invalid/3mf/metadata/v1\" unit=\"millimeter\">");
    metadata(&mut xml, "EngineVersion", &request.engine_version);
    metadata(&mut xml, "SourceName", &source.source_name);
    metadata(&mut xml, "SourceAttribution", &source.attribution);
    metadata(&mut xml, "SourceLicense", &source.license_identifier);
    metadata(
        &mut xml,
        "CanonicalGeometrySHA256",
        &prepared.canonical_geometry_sha256,
    );
    metadata(&mut xml, "ProvenanceSHA256", &prepared.provenance_sha256);
    xml.push_str("<resources><object id=\"1\" type=\"model\"><mesh><vertices>");
    for vertex in &mesh.vertices_mm {
        if !vertex
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
        {
            return Err(invalid("nonfinite or unsupported 3MF coordinate"));
        }
        xml.push_str(&format!(
            "<vertex x=\"{:.12}\" y=\"{:.12}\" z=\"{:.12}\"/>",
            vertex[0], vertex[1], vertex[2]
        ));
    }
    xml.push_str("</vertices><triangles>");
    for [a, b, c] in &mesh.triangles {
        xml.push_str(&format!("<triangle v1=\"{a}\" v2=\"{b}\" v3=\"{c}\"/>"));
    }
    xml.push_str(
        "</triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>",
    );
    if xml.len() > MAX_PART_SIZE {
        return Err(ExportError::new(
            "OUT_OF_MEMORY",
            "3MF XML model exceeds 64MiB",
        ));
    }
    Ok(xml)
}

fn zip_add(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    content: &[u8],
) -> Result<(), ExportError> {
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    zip.start_file(name, opts)
        .map_err(|e| invalid(format!("zip part {name}: {e}")))?;
    zip.write_all(content)
        .map_err(|e| invalid(format!("zip write {name}: {e}")))
}

/// Build a real in-memory .3mf package, then parse the produced ZIP and mesh
/// independently. No bytes or success result leave this function on failure.
pub fn export_3mf(
    result: &BuildOutput,
    grid: &ElevationGridV1,
    request: &TerrainBuildRequestV1,
    source: &TerrainSourceManifestV1,
) -> Result<Exported3mf, ExportError> {
    let prepared = prepare_export(result, grid, request, source)?;
    let xml = model_xml(&result.mesh, &prepared, source, request)?;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip_add(&mut zip, "[Content_Types].xml", CONTENT_TYPES.as_bytes())?;
    zip_add(&mut zip, "_rels/.rels", RELATIONSHIPS.as_bytes())?;
    zip_add(&mut zip, "3D/3dmodel.model", xml.as_bytes())?;
    zip_add(
        &mut zip,
        "Metadata/map2model.provenance.json",
        &prepared.provenance_bytes,
    )?;
    let bytes = zip
        .finish()
        .map_err(|e| invalid(format!("ZIP finish: {e}")))?
        .into_inner();
    if bytes.len() > MAX_PACKAGE_SIZE {
        return Err(ExportError::new(
            "OUT_OF_MEMORY",
            "3MF archive exceeds 80MiB",
        ));
    }
    let inspected = inspect_3mf(&bytes, result, &prepared)?;
    Ok(Exported3mf {
        bytes,
        package_sha256: inspected.package_sha256,
        provenance_sha256: inspected.provenance_sha256,
        canonical_geometry_sha256: inspected.canonical_geometry_sha256,
        vertices: inspected.vertices,
        triangles: inspected.triangles,
        aabb_mm: inspected.aabb_mm,
    })
}

fn read_part(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>, ExportError> {
    let mut entry = zip
        .by_name(name)
        .map_err(|_| invalid(format!("3MF missing required part {name}")))?;
    let n = usize::try_from(entry.size()).map_err(|_| invalid("oversize 3MF part"))?;
    if n > MAX_PART_SIZE || entry.compression() != CompressionMethod::Stored {
        return Err(invalid(format!(
            "unsupported compression or oversized 3MF part {name}"
        )));
    }
    let mut content = Vec::with_capacity(n);
    entry
        .read_to_end(&mut content)
        .map_err(|e| invalid(format!("ZIP read: {e}")))?;
    if content.len() != n {
        return Err(invalid("3MF part size mismatch"));
    }
    Ok(content)
}
fn parse_xml(xml: &str) -> Result<Document<'_>, ExportError> {
    Document::parse(xml).map_err(|e| invalid(format!("invalid XML/DTD in 3MF: {e}")))
}
fn required_attr<'a, 'input>(
    n: roxmltree::Node<'a, 'input>,
    attr: &str,
) -> Result<&'a str, ExportError> {
    n.attribute(attr)
        .ok_or_else(|| invalid(format!("3MF XML required attribute {attr} absent")))
}
fn exactly_one<'a, 'input>(
    doc: &'a Document<'input>,
    namespace: &str,
    name: &str,
) -> Result<roxmltree::Node<'a, 'input>, ExportError> {
    let mut nodes = doc.descendants().filter(|n| {
        n.is_element() && n.tag_name().namespace() == Some(namespace) && n.tag_name().name() == name
    });
    let node = nodes
        .next()
        .ok_or_else(|| invalid(format!("3MF missing <{name}>")))?;
    if nodes.next().is_some() {
        return Err(invalid(format!("3MF multiple <{name}> elements")));
    }
    Ok(node)
}
fn aabb(points: &[[f64; 3]]) -> [f64; 6] {
    let mut bb = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in points {
        for axis in 0..3 {
            bb[axis] = bb[axis].min(p[axis]);
            bb[axis + 3] = bb[axis + 3].max(p[axis]);
        }
    }
    bb
}

/// Independent package reader verifies OPC, part relationships/content-types,
/// model XML, mesh indices and geometry/provenance SHA values and dimensions.
/// This is a strict project-specific structural validator, *not* the official
/// 3MF Consortium XSD/conformance suite or a slicer acceptance report.
pub fn inspect_3mf(
    bytes: &[u8],
    original: &BuildOutput,
    prepared: &PreparedExport,
) -> Result<Inspected3mf, ExportError> {
    if bytes.len() < 100 || bytes.len() > MAX_PACKAGE_SIZE {
        return Err(invalid("3MF ZIP byte length invalid"));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| invalid(format!("3MF ZIP invalid: {e}")))?;
    if zip.len() != 4 {
        return Err(invalid("3MF requires exactly four declared OPC parts"));
    }
    let expected = [
        "[Content_Types].xml",
        "_rels/.rels",
        "3D/3dmodel.model",
        "Metadata/map2model.provenance.json",
    ];
    for n in 0..zip.len() {
        let f = zip
            .by_index(n)
            .map_err(|e| invalid(format!("3MF bad ZIP index: {e}")))?;
        if !expected.contains(&f.name()) {
            return Err(invalid("3MF unexpected/duplicate ZIP part"));
        }
    }
    let content = read_part(&mut zip, expected[0])?;
    let relationships = read_part(&mut zip, expected[1])?;
    let model = read_part(&mut zip, expected[2])?;
    let provenance = read_part(&mut zip, expected[3])?;
    if provenance != prepared.provenance_bytes {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "3MF embedded provenance differs",
        ));
    }
    let provenance_sha256 = format!("{:x}", Sha256::digest(&provenance));
    if provenance_sha256 != prepared.provenance_sha256 {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "3MF provenance digest mismatch",
        ));
    }
    serde_json::from_slice::<serde_json::Value>(&provenance)
        .map_err(|e| invalid(format!("invalid provenance JSON: {e}")))?;
    let content_xml =
        std::str::from_utf8(&content).map_err(|_| invalid("non-UTF8 OPC Content Types"))?;
    let types = parse_xml(content_xml)?;
    if !types.root_element().has_tag_name((OPC_TYPES_NS, "Types")) {
        return Err(invalid("invalid OPC Types root"));
    }
    for (path, mime) in [
        (MODEL_PART, MODEL_TYPE),
        (PROVENANCE_PART, "application/json"),
    ] {
        let ok = types.descendants().any(|node| {
            node.has_tag_name((OPC_TYPES_NS, "Override"))
                && node.attribute("PartName") == Some(path)
                && node.attribute("ContentType") == Some(mime)
        });
        if !ok {
            return Err(invalid(format!("3MF missing OPC Content Type {path}")));
        }
    }
    let rel_xml =
        std::str::from_utf8(&relationships).map_err(|_| invalid("non-UTF8 OPC relationships"))?;
    let rels = parse_xml(rel_xml)?;
    if !rels
        .root_element()
        .has_tag_name((OPC_RELS_NS, "Relationships"))
    {
        return Err(invalid("3MF bad OPC relationships root"));
    }
    for (target, kind) in [(MODEL_PART, START_REL), (PROVENANCE_PART, PRESERVE_REL)] {
        let count = rels
            .descendants()
            .filter(|n| {
                n.has_tag_name((OPC_RELS_NS, "Relationship"))
                    && n.attribute("Target") == Some(target)
                    && n.attribute("Type") == Some(kind)
            })
            .count();
        if count != 1 {
            return Err(invalid(format!(
                "missing/multiple OPC relationship for {target}"
            )));
        }
    }
    let model_xml = std::str::from_utf8(&model).map_err(|_| invalid("non-UTF8 3MF model"))?;
    let doc = parse_xml(model_xml)?;
    let root = doc.root_element();
    if !root.has_tag_name((CORE_NS, "model")) || root.attribute("unit") != Some("millimeter") {
        return Err(invalid("3MF not Core model in millimeters"));
    }
    let mesh_node = exactly_one(&doc, CORE_NS, "mesh")?;
    let object = exactly_one(&doc, CORE_NS, "object")?;
    if object.attribute("id") != Some("1")
        || object.attribute("type") != Some("model")
        || !mesh_node.ancestors().any(|parent| parent == object)
    {
        return Err(invalid("3MF build resource object is invalid"));
    }
    let build = exactly_one(&doc, CORE_NS, "build")?;
    let items = build
        .children()
        .filter(|n| n.has_tag_name((CORE_NS, "item")))
        .collect::<Vec<_>>();
    if items.len() != 1
        || items[0].attribute("objectid") != Some("1")
        || items[0].attribute("transform").is_some()
    {
        return Err(invalid("3MF build item/transform is invalid"));
    }
    let vertex_root = exactly_one(&doc, CORE_NS, "vertices")?;
    let triangle_root = exactly_one(&doc, CORE_NS, "triangles")?;
    if !vertex_root.ancestors().any(|parent| parent == mesh_node)
        || !triangle_root.ancestors().any(|parent| parent == mesh_node)
    {
        return Err(invalid("3MF mesh vertices/triangles not attached to mesh"));
    }
    let mut vertices = Vec::new();
    for n in vertex_root.children().filter(|n| n.is_element()) {
        if !n.has_tag_name((CORE_NS, "vertex")) {
            return Err(invalid("unexpected XML element in 3MF vertices"));
        }
        let mut point = [0.0; 3];
        for (i, key) in ["x", "y", "z"].iter().enumerate() {
            point[i] = required_attr(n, key)?
                .parse::<f64>()
                .map_err(|_| invalid("3MF malformed vertex coordinate"))?;
            if !point[i].is_finite() {
                return Err(invalid("3MF nonfinite vertex coordinate"));
            }
        }
        vertices.push(point);
        if vertices.len() > 200_000 {
            return Err(ExportError::new(
                "OUT_OF_MEMORY",
                "3MF vertex limit exceeded",
            ));
        }
    }
    let mut triangles = Vec::new();
    for n in triangle_root.children().filter(|n| n.is_element()) {
        if !n.has_tag_name((CORE_NS, "triangle")) {
            return Err(invalid("unexpected XML element in 3MF triangles"));
        }
        let mut indices = [0usize; 3];
        for (i, key) in ["v1", "v2", "v3"].iter().enumerate() {
            indices[i] = required_attr(n, key)?
                .parse::<usize>()
                .map_err(|_| invalid("3MF malformed triangle index"))?;
        }
        if indices.iter().any(|i| *i >= vertices.len())
            || indices[0] == indices[1]
            || indices[1] == indices[2]
            || indices[0] == indices[2]
        {
            return Err(invalid("3MF invalid/repeated vertex indices"));
        }
        triangles.push(indices);
        if triangles.len() > 400_000 {
            return Err(ExportError::new(
                "OUT_OF_MEMORY",
                "3MF triangle limit exceeded",
            ));
        }
    }
    let bbox = aabb(&vertices);
    if vertices.len() != original.geometry.vertices
        || triangles.len() != original.geometry.triangles
        || bbox
            .iter()
            .zip(original.geometry.measured_aabb_mm.iter())
            .any(|(a, b)| (a - b).abs() > 0.00001)
    {
        return Err(invalid(
            "3MF readback mesh dimensions/count differ from input",
        ));
    }
    let restored = TerrainMesh {
        vertices_mm: vertices,
        triangles,
        columns: original.mesh.columns,
        rows: original.mesh.rows,
    };
    map2model_mesh::inspect_mesh(&restored)?;
    let canonical_tris = restored
        .triangles
        .iter()
        .map(|t| MeshTriangle {
            part_id: "terrain".to_owned(),
            indices: *t,
        })
        .collect::<Vec<_>>();
    let geometry_sha = canonical_mesh_digest(&restored.vertices_mm, &canonical_tris)
        .map_err(|e| invalid(format!("3MF mesh SHA encoding: {e}")))?;
    if geometry_sha != prepared.canonical_geometry_sha256
        || geometry_sha != original.canonical_geometry_sha256
    {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "3MF parsed geometry digest differs",
        ));
    }
    Ok(Inspected3mf {
        package_sha256: format!("{:x}", Sha256::digest(bytes)),
        provenance_sha256,
        canonical_geometry_sha256: geometry_sha,
        vertices: restored.vertices_mm.len(),
        triangles: restored.triangles.len(),
        aabb_mm: bbox,
    })
}

/// Native-only atomic no-clobber publish *after* all ZIP readback checks.
/// A cancellation check occurs both before staging and just before linking.
/// Caller must provide a previously authorized, same-filesystem output path.
/// This function does not accept raw renderer paths and does not generate
/// JobStateV1 success messages; the native IPC supervisor owns those.
pub fn write_3mf_atomic(
    destination: &Path,
    artifact: &Exported3mf,
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<(), ExportError> {
    if is_cancelled() {
        return Err(ExportError::new(
            "CANCELLED",
            "export cancelled before publication",
        ));
    }
    if destination.extension().and_then(|x| x.to_str()) != Some("3mf") {
        return Err(invalid("destination must end in .3mf"));
    }
    if destination.exists() {
        return Err(invalid("refusing to overwrite an existing artifact"));
    }
    if format!("{:x}", Sha256::digest(&artifact.bytes)) != artifact.package_sha256 {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "artifact bytes changed after validation",
        ));
    }
    let file_name = destination
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| invalid("missing or non-UTF8 export name"))?;
    // Each *attempt* has a fresh OS-backed 128-bit random stage identity;
    // unlike the old PID+archive-SHA name, a crash can leave a prior stage
    // without preventing a later retry. create_new is the final exclusivity
    // guard even if the nonce collides or another process races us.
    let mut reserved = None;
    for _ in 0..8 {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce)
            .map_err(|e| invalid(format!("OS random staging identity failed: {e}")))?;
        let token = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = destination.with_file_name(format!(
            ".{file_name}.{}.{token}.staging",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
        {
            Ok(file) => {
                reserved = Some((path, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(invalid(format!("staging create: {error}"))),
        }
    }
    let (temporary, mut file) = reserved.ok_or_else(|| invalid("no unused random staging name"))?;
    let stage_for_link = temporary.as_path();
    // Same-directory hard link is an atomic, no-clobber commit on Windows
    // and Unix. Move the file handle into this closure so it closes even on
    // write/sync failure before removing our *own* temporary file.
    let outcome = (move || -> Result<(), ExportError> {
        file.write_all(&artifact.bytes)
            .map_err(|e| invalid(format!("staging write: {e}")))?;
        file.sync_all()
            .map_err(|e| invalid(format!("staging sync: {e}")))?;
        drop(file);
        if is_cancelled() {
            return Err(ExportError::new(
                "CANCELLED",
                "export cancelled before publication",
            ));
        }
        std::fs::hard_link(stage_for_link, destination)
            .map_err(|e| invalid(format!("atomic no-clobber publication: {e}")))?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporary);
    outcome
}

/// G2 local-only experimental ZIP branch. This accepts NO verified source
/// manifest and never changes the strict verified export_3mf preflight.
pub fn export_experimental_core(
    result: &BuildOutput,
    provenance: &serde_json::Value,
) -> Result<Exported3mf, ExportError> {
    let field = |k: &str| provenance.get(k);
    for (k, v) in [
        ("experimentalOnly", true),
        ("localPersonalUseOnly", true),
        ("ownerAcknowledgedUnverified", true),
        ("encodingAuthenticated", false),
        ("sourceLicenseVerified", false),
        ("nativeResolutionVerified", false),
        ("redistributionApproved", false),
        ("productionGenerationReady", false),
    ] {
        if field(k).and_then(serde_json::Value::as_bool) != Some(v) {
            return Err(invalid(format!(
                "experimental warning flag {k} is missing or forged"
            )));
        }
    }
    if field("verticalDatumStatus").and_then(serde_json::Value::as_str) != Some("unknown")
        || field("kind").and_then(serde_json::Value::as_str) != Some("Experimental3mfProvenanceV1")
        || field("acknowledgementPolicyVersion")
            .and_then(serde_json::Value::as_str)
            .is_none()
    {
        return Err(invalid(
            "unverified source provenance disclosure is missing",
        ));
    }
    for key in [
        "sourceArchiveSha256",
        "candidateHeightsSha256",
        "jobId",
        "selectionId",
    ] {
        if field(key)
            .and_then(serde_json::Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(invalid(format!("missing candidate provenance {key}")));
        }
    }
    let source_sha = field("sourceArchiveSha256")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let sample_sha = field("candidateHeightsSha256")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if [source_sha, sample_sha].iter().any(|s| {
        s.len() != 64
            || !s
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }) {
        return Err(invalid("candidate source/sample hash is malformed"));
    }
    let geometry = map2model_mesh::inspect_mesh(&result.mesh)?;
    if geometry.vertices != result.geometry.vertices
        || geometry.triangles != result.geometry.triangles
        || !geometry.closed
        || !geometry.oriented
        || geometry.non_manifold_edges != 0
    {
        return Err(invalid("candidate mesh is not validated"));
    }
    let triangles = result
        .mesh
        .triangles
        .iter()
        .map(|t| MeshTriangle {
            part_id: "terrain".into(),
            indices: *t,
        })
        .collect::<Vec<_>>();
    let canonical = canonical_mesh_digest(&result.mesh.vertices_mm, &triangles)
        .map_err(|e| invalid(format!("experimental geometry hash failed: {e}")))?;
    if canonical != result.canonical_geometry_sha256 {
        return Err(ExportError::new(
            "HASH_MISMATCH",
            "experimental geometry identity changed",
        ));
    }
    let mut provenance = provenance.clone();
    provenance["canonicalGeometrySha256"] = serde_json::Value::String(canonical.clone());
    let bytes_provenance = serde_json::to_vec(&provenance)
        .map_err(|_| invalid("cannot encode experimental provenance"))?;
    if bytes_provenance.len() > 16 * 1024 {
        return Err(invalid("experimental source provenance too large"));
    }
    let provenance_sha = format!("{:x}", Sha256::digest(&bytes_provenance));
    let warning = "EXPERIMENTAL — USER-SUPPLIED TERRAIN. Source permissions not independently verified; vertical datum and native accuracy unknown. Private local evaluation only; third-party license and attribution requirements still apply. Not a certified elevation model. Redistribution not approved by Map2Model Next.";
    let attribution = field("sourceAttributionFromArchive")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Source attribution not provided in archive");
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml.push_str("<model xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" unit=\"millimeter\">");
    metadata(
        &mut xml,
        "SourceStatus",
        "EXPERIMENTAL — PRIVATE LOCAL USE — UNVERIFIED",
    );
    metadata(&mut xml, "SourceWarning", warning);
    metadata(&mut xml, "VerticalDatumStatus", "unknown");
    metadata(
        &mut xml,
        "SourceRightsStatus",
        "unknown; not independently verified",
    );
    metadata(&mut xml, "SourceAttribution", attribution);
    metadata(&mut xml, "SourceArchiveSHA256", source_sha);
    metadata(&mut xml, "CandidateHeightsSHA256", sample_sha);
    metadata(&mut xml, "CanonicalGeometrySHA256", &canonical);
    metadata(&mut xml, "ProvenanceSHA256", &provenance_sha);
    xml.push_str("<resources><object id=\"1\" type=\"model\"><mesh><vertices>");
    for v in &result.mesh.vertices_mm {
        if !v.iter().all(|x| x.is_finite() && x.abs() < 1_000_000.) {
            return Err(invalid("invalid candidate vertex coordinate"));
        }
        xml.push_str(&format!(
            "<vertex x=\"{:.12}\" y=\"{:.12}\" z=\"{:.12}\"/>",
            v[0], v[1], v[2]
        ));
    }
    xml.push_str("</vertices><triangles>");
    for [a, b, c] in &result.mesh.triangles {
        xml.push_str(&format!("<triangle v1=\"{a}\" v2=\"{b}\" v3=\"{c}\"/>"));
    }
    xml.push_str(
        "</triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>",
    );
    if xml.len() > MAX_PART_SIZE {
        return Err(invalid("experimental Core model XML oversized"));
    }
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip_add(&mut zip, "[Content_Types].xml", CONTENT_TYPES.as_bytes())?;
    zip_add(&mut zip, "_rels/.rels", RELATIONSHIPS.as_bytes())?;
    zip_add(&mut zip, "3D/3dmodel.model", xml.as_bytes())?;
    zip_add(
        &mut zip,
        "Metadata/map2model.provenance.json",
        &bytes_provenance,
    )?;
    let bytes = zip
        .finish()
        .map_err(|e| invalid(format!("experimental ZIP: {e}")))?
        .into_inner();
    if bytes.len() > MAX_PACKAGE_SIZE {
        return Err(invalid("experimental 3MF exceeds ZIP budget"));
    }
    let prepared = PreparedExport {
        provenance_bytes: bytes_provenance,
        provenance_sha256: provenance_sha,
        canonical_geometry_sha256: canonical,
        model_part: MODEL_PART,
        provenance_part: PROVENANCE_PART,
        core_spec_version: "1.4.0",
    };
    let inspected = inspect_3mf(&bytes, result, &prepared)?;
    // The independent readback must retain the mandatory warning in Core XML.
    let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice()))
        .map_err(|_| invalid("cannot independently read experimental ZIP"))?;
    let model = read_part(&mut archive, "3D/3dmodel.model")?;
    let text = std::str::from_utf8(&model).map_err(|_| invalid("non-UTF8 candidate XML"))?;
    if !text.contains("EXPERIMENTAL")
        || !text.contains("unknown")
        || !text.contains(&provenance_sha)
    {
        return Err(invalid("experimental Core XML lost disclaimer"));
    }
    Ok(Exported3mf {
        bytes,
        package_sha256: inspected.package_sha256,
        provenance_sha256: inspected.provenance_sha256,
        canonical_geometry_sha256: inspected.canonical_geometry_sha256,
        vertices: inspected.vertices,
        triangles: inspected.triangles,
        aabb_mm: inspected.aabb_mm,
    })
}
