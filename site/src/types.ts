import type { DestinationField, Mapping, Source } from "@arlyon/f-ck-wasm";

export interface NodeData {
  label: string;
}

export interface SourceNodeData extends NodeData {
  source: Source;
}

export interface MappingNodeData extends NodeData {
  mapping: Mapping;
}

export interface DestinationNodeData extends NodeData {
  field: DestinationField;
}
