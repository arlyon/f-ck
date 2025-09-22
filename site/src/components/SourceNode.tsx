import { Handle, Position } from '@xyflow/react';
import { SourceNodeData } from '../types';

interface SourceNodeProps {
  data: SourceNodeData;
}

export default function SourceNode({ data }: SourceNodeProps) {
  return (
    <div className="node-content">
      <div className="node-title">{data.source.path}</div>
      <div className="node-subtitle">{data.source.format} • {data.source.id}</div>
      <Handle 
        type="source" 
        position={Position.Right} 
        style={{ background: '#2196f3' }}
      />
    </div>
  );
}