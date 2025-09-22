import { Handle, Position } from '@xyflow/react';
import { MappingNodeData } from '../types';

interface MappingNodeProps {
  data: MappingNodeData;
}

export default function MappingNode({ data }: MappingNodeProps) {
  const policyLabel = data.mapping.policy.type === 'firstMatch' 
    ? `First Match${data.mapping.policy.priority ? ` (${data.mapping.policy.priority.join(', ')})` : ''}`
    : data.mapping.policy.type;

  return (
    <div className="node-content">
      <Handle 
        type="target" 
        position={Position.Left}
        style={{ background: '#9c27b0' }}
      />
      <div className="node-title">{data.mapping.destination_field}</div>
      <div className="node-subtitle">{policyLabel}</div>
      <div className="node-subtitle">
        {data.mapping.source_fields.length} source field{data.mapping.source_fields.length !== 1 ? 's' : ''}
      </div>
      <Handle 
        type="source" 
        position={Position.Right}
        style={{ background: '#9c27b0' }}
      />
    </div>
  );
}