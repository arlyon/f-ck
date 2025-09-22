import { Plus, Database, ArrowRight, Target } from 'lucide-react';
import { Source, DestinationField, Mapping } from '../types';

interface SidebarProps {
  sources: Source[];
  destinationFields: DestinationField[];
  mappings: Mapping[];
  onAddSource: () => void;
  onAddDestinationField: () => void;
  onAddMapping: () => void;
}

export default function Sidebar({ 
  sources, 
  destinationFields, 
  mappings,
  onAddSource,
  onAddDestinationField,
  onAddMapping
}: SidebarProps) {
  return (
    <div className="sidebar">
      <h2 style={{ marginBottom: '20px', fontSize: '18px', fontWeight: 'bold' }}>
        Query Builder
      </h2>
      
      <div style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
          <Database size={16} />
          <h3 style={{ fontSize: '14px', fontWeight: '600' }}>Sources</h3>
          <button
            onClick={onAddSource}
            style={{
              marginLeft: 'auto',
              padding: '4px',
              border: 'none',
              background: '#2196f3',
              color: 'white',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            <Plus size={14} />
          </button>
        </div>
        {sources.map(source => (
          <div
            key={source.id}
            draggable
            onDragStart={(e) => {
              e.dataTransfer.setData('application/reactflow', JSON.stringify({
                type: 'source',
                data: source
              }));
            }}
            style={{
              padding: '8px',
              background: '#e3f2fd',
              border: '1px solid #2196f3',
              borderRadius: '4px',
              marginBottom: '8px',
              cursor: 'grab',
              fontSize: '12px'
            }}
          >
            <div style={{ fontWeight: '500' }}>{source.path}</div>
            <div style={{ color: '#666' }}>{source.format}</div>
          </div>
        ))}
      </div>

      <div style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
          <ArrowRight size={16} />
          <h3 style={{ fontSize: '14px', fontWeight: '600' }}>Mappings</h3>
          <button
            onClick={onAddMapping}
            style={{
              marginLeft: 'auto',
              padding: '4px',
              border: 'none',
              background: '#9c27b0',
              color: 'white',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            <Plus size={14} />
          </button>
        </div>
        {mappings.map((mapping, index) => (
          <div
            key={index}
            draggable
            onDragStart={(e) => {
              e.dataTransfer.setData('application/reactflow', JSON.stringify({
                type: 'mapping',
                data: mapping
              }));
            }}
            style={{
              padding: '8px',
              background: '#f3e5f5',
              border: '1px solid #9c27b0',
              borderRadius: '4px',
              marginBottom: '8px',
              cursor: 'grab',
              fontSize: '12px'
            }}
          >
            <div style={{ fontWeight: '500' }}>{mapping.destination_field}</div>
            <div style={{ color: '#666' }}>{mapping.policy.type}</div>
          </div>
        ))}
      </div>

      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
          <Target size={16} />
          <h3 style={{ fontSize: '14px', fontWeight: '600' }}>Destination</h3>
          <button
            onClick={onAddDestinationField}
            style={{
              marginLeft: 'auto',
              padding: '4px',
              border: 'none',
              background: '#4caf50',
              color: 'white',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            <Plus size={14} />
          </button>
        </div>
        {destinationFields.map((field, index) => (
          <div
            key={index}
            draggable
            onDragStart={(e) => {
              e.dataTransfer.setData('application/reactflow', JSON.stringify({
                type: 'destination',
                data: field
              }));
            }}
            style={{
              padding: '8px',
              background: '#e8f5e8',
              border: '1px solid #4caf50',
              borderRadius: '4px',
              marginBottom: '8px',
              cursor: 'grab',
              fontSize: '12px'
            }}
          >
            <div style={{ fontWeight: '500' }}>{field.name}</div>
            <div style={{ color: '#666' }}>{field.data_type}</div>
          </div>
        ))}
      </div>
    </div>
  );
}