/** Map2Model Next V1 contract proposal. JSON Schema is normative until G0 ratification.
 * TS declarations describe shapes; runtime validation is mandatory.
 */
export type SchemaVersionV1 = "1.0.0";
export type Sha256 = string;
export type Bbox4326 = [west: number, south: number, east: number, north: number];
export type LocalRectMeters = [minEast: number, minNorth: number, maxEast: number, maxNorth: number];

export interface SelectionV1 {
  schemaVersion: SchemaVersionV1; kind: "SelectionV1"; selectionId: string;
  planet: "earth"; horizontalCrs: "EPSG:4326"; bbox: Bbox4326;
  terrainProfile: { targetGroundSampleMeters: number; noDataPolicy: "fail"; maxSamples: number };
}
export interface TerrainSourceManifestV1 {
  schemaVersion: SchemaVersionV1; kind: "TerrainSourceManifestV1";
  sourceName: string; releaseId: string; archiveId: string;
  localAssetToken: string; sourceArchiveSha256: Sha256; manifestSha256: Sha256;
  coverageBbox: Bbox4326; coverageStatus: "envelope-only" | "verified-tile-index";
  coverageIndexSha256: Sha256 | null; verticalDatumStatus: "known" | "unknown" | "unsupported";
  horizontalCrs: string; verticalDatumId: string; nominalGroundSampleMeters: number;
  nativeResolutionSource: string; uncertaintyMeters: number | null;
  sourceBytes: number; licenseIdentifier: string; attribution: string; decoderVersion: string;
}
export interface ElevationGridV1 {
  schemaVersion: SchemaVersionV1; kind: "ElevationGridV1";
  gridId: string; selectionId: string; sourceManifestSha256: Sha256;
  width: number; height: number; samplesMeters: Array<number | null> | null;
  validMask: boolean[] | null; storageMode: "inline-fixture" | "local-binary-token";
  gridDataToken: string | null; gridDataSha256: Sha256 | null;
  minEastMeters: number; maxNorthMeters: number;
  dxMeters: number; dyMeters: number; anchorWgs84: [longitude: number, latitude: number];
  horizontalTransformId: "WGS84-ENU-LINEARIZED-V1"; verticalDatumId: string;
  storageConvention: "north-up-f64-row-major"; gridSha256: Sha256;
}
export interface TerrainBuildRequestV1 {
  schemaVersion: SchemaVersionV1; kind: "TerrainBuildRequestV1";
  jobId: string; correlationId: string; selectionId: string; gridSha256: Sha256;
  clipRectMeters: LocalRectMeters; modelWidthMm: number; modelHeightMm: number;
  baseThicknessMm: number; minimumFeatureThicknessMm: number;
  verticalExaggeration: number; elevationZeroMeters: number;
  engineVersion: string; exportToken: string;
}
export interface MeshAnd3mfResultV1 {
  schemaVersion: SchemaVersionV1; kind: "MeshAnd3mfResultV1"; jobId: string;
  geometry: {
    units: "millimeter"; vertices: number; triangles: number; closed: boolean;
    oriented: boolean; nonManifoldEdges: number; degenerateTriangles: number;
    minimumMeasuredThicknessMm: number;
    measuredAabbMm: [number,number,number,number,number,number];
    componentCount: number; partCount: number; volumeMm3: number;
    selfIntersectionStatus: "passed" | "failed" | "not-checked";
    validation: {status:"passed" | "failed"; checks:string[];
      diagnostics:Array<{code:string;message:string;elementIndex:number|null}>};
  };
  artifact: {
    exportToken: string; sha256: Sha256; sizeBytes: number; format: "3mf";
    conformance: "passed" | "failed"; coreSpecVersion: "1.4.0";
    modelPart: "/3D/3dmodel.model";
    provenancePart: "/Metadata/map2model.provenance.json";
    validatorId: string; validatorVersion: string;
    diagnostics: Array<{code:string;message:string}>;
  };
  canonicalGeometrySha256: Sha256; provenanceSha256: Sha256; warnings: string[];
}
export type JobReasonCodeV1 =
  | "DATA_MISSING" | "INVALID_REQUEST" | "INVALID_SELECTION" | "INCOMPLETE_GRID" | "INVALID_GRID_HASH" | "INVALID_GEOMETRY" | "INVALID_DIMENSIONS" | "NONFINITE_ELEVATION" | "INVALID_CRS" | "UNSUPPORTED_DATUM"
  | "HASH_MISMATCH" | "INVALID_SOURCE" | "OUT_OF_MEMORY"
  | "NON_MANIFOLD_MESH" | "EXPORT_FAILED" | "CANCELLED"
  | "UNSUPPORTED_OPERATION" | "INTERNAL_ERROR";
export interface JobStateV1 {
  schemaVersion: SchemaVersionV1; kind: "JobStateV1";
  jobId: string; correlationId: string;
  phase: "queued" | "import" | "decode" | "mesh" | "export" | "verify" | "complete" | "cancel";
  status: "queued" | "running" | "cancel_requested" | "succeeded" | "failed" | "cancelled";
  progress01: number; reasonCode: JobReasonCodeV1 | null; message: string;
  details: {retryable:boolean;missingTiles:string[];
    invalidSampleIndex:number|null;invalidMeshElementIndex:number|null};
}
export type ContractV1 = SelectionV1 | TerrainSourceManifestV1 | ElevationGridV1 |
  TerrainBuildRequestV1 | MeshAnd3mfResultV1 | JobStateV1;
