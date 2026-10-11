#!/usr/bin/env python3
"""Validate Rust-emitted 3MF model XML using the published official Core 1.4 XSD.

Vendor snapshot: 3MFConsortium/spec_core 997b385... Appendix B.1.1.
This validates *Core model XML* with official schema using lxml; independent
OPC archive/relationship, mesh canonical hash, and slicer checks run separately.
"""
import hashlib
import json
import pathlib
import sys
import zipfile

from lxml import etree


class OfflineW3CResolver(etree.Resolver):
    def resolve(self, url, public_id, context):
        if url == "http://www.w3.org/2001/xml.xsd":
            return self.resolve_filename(
                str(pathlib.Path(__file__).parent / "official_core_1_4" / "xml.xsd"),
                context
            )
        raise ValueError(f"Unexpected external XSD import: {url}")


def verify(file_name):
    folder = pathlib.Path(__file__).parent / "official_core_1_4"
    schema_bytes = (folder / "3mf-core-1.4.xsd").read_bytes()
    # 3MF Core Appendix B.1.1 publishes maxOccurs="2147483647".
    # libxml2/lxml rejects that implementation-limit value while XML Schema
    # allows it. Normalize to "unbounded" solely for parser compatibility.
    # Every supported G1 mesh is many orders of magnitude below that cap.
    # Preserve and hash the exact unmodified official schema separately.
    replacement_count = schema_bytes.count(b'maxOccurs="2147483647"')
    if replacement_count < 1:
        raise ValueError("Pinned Consortium schema no longer has expected maxOccurs bounds")
    schema_for_lxml = schema_bytes.replace(
        b'maxOccurs="2147483647"', b'maxOccurs="unbounded"'
    )
    parser = etree.XMLParser(no_network=True, resolve_entities=False, load_dtd=False)
    parser.resolvers.add(OfflineW3CResolver())
    schema_tree = etree.ElementTree(etree.fromstring(schema_for_lxml, parser))
    schema = etree.XMLSchema(schema_tree)
    with zipfile.ZipFile(file_name) as archive:
        model_bytes = archive.read("3D/3dmodel.model")
    model_tree = etree.fromstring(model_bytes, parser)
    schema.assertValid(model_tree)
    return {
        "validatorId": "consortium-published-core-1.4-xsd",
        "validatorVersion": etree.LXML_VERSION,
        "validatorScope": "Consortium Core 1.4 XSD with transparently recorded libxml2 maxOccurs compatibility normalization; separate OPC/slicer checks required",
        "schemaSnapshotCommit": "997b385e06f3181cf9aae0c578e0b45ccd48ccb2",
        "schemaSha256": hashlib.sha256(schema_bytes).hexdigest(),
        "normalizedSchemaSha256": hashlib.sha256(schema_for_lxml).hexdigest(),
        "maxOccursCompatibilityReplacements": replacement_count,
        "modelXmlSha256": hashlib.sha256(model_bytes).hexdigest(),
        "packageSha256": hashlib.sha256(pathlib.Path(file_name).read_bytes()).hexdigest(),
        "status": "passed",
    }


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("Usage: core_xsd_check.py PATH_TO_RUST_OUTPUT.3mf")
        print(json.dumps(verify(sys.argv[1]), sort_keys=True, separators=(",", ":")))
    except (OSError, ValueError, etree.XMLSyntaxError, etree.XMLSchemaError,
            etree.DocumentInvalid, zipfile.BadZipFile) as error:
        print(f"3MF Core 1.4 XSD validation FAILED: {error}", file=sys.stderr)
        sys.exit(1)
