import { Handle, Position } from '@xyflow/react';
import { DestinationNodeData } from '../types';

interface DestinationNodeProps {
  data: DestinationNodeData;
}

export default function DestinationNode({ data }: DestinationNodeProps) {
  return (
    <div className="node-content">
      <Handle 
        type="target" 
        position={Position.Left}
        style={{ background: '#4caf50' }}
      />
      <div className="node-title">{data.field.name}</div>
      <div className="node-subtitle">{data.field.data_type}</div>
    </div>
  );
}