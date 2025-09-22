import {
	addEdge,
	Background,
	BackgroundVariant,
	type Connection,
	Controls,
	Edge,
	MiniMap,
	type Node,
	type NodeTypes,
	ReactFlow,
	useEdgesState,
	useNodesState,
} from "@xyflow/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "@xyflow/react/dist/style.css";
import type {
	DestinationField,
	Mapping,
	QueryPlan,
	Source,
} from "@arlyon/f-ck-wasm";
import init, { execute_query_json, SourceHandle } from "@arlyon/f-ck-wasm";
import {
	forceManyBody,
	forceSimulation,
	forceX,
	type SimulationNodeDatum,
} from "d3-force";
import type { JSONSchema7 } from "json-schema";
import DestinationNode from "./components/DestinationNode";
import { GroupNode } from "./components/labeled-group-node";
import MappingNode from "./components/MappingNode";
import SourceNode from "./components/SourceNode";

const nodeTypes: NodeTypes = {
	source: SourceNode,
	mapping: MappingNode,
	destination: DestinationNode,
	labeledGroupNode: GroupNode,
};

await init();

export default function App() {
	const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
	const [edges, setEdges, onEdgesChange] = useEdgesState([]);
	const [sources, setSources] = useState<
		{ handle: SourceHandle; schema: JSONSchema7 }[]
	>([]);
	const [destinationFields, setDestinationFields] = useState<
		DestinationField[]
	>([
		{
			data_type: "string",
			name: "customer_id",
		},
	]);
	const [mappings, setMappings] = useState<Mapping[]>([
		{
			destination_field: "customer_id",
			policy: {
				type: "firstMatch",
			},
			source_fields: [
				{
					column_name: "id",
					source_file_id: "olist_customers_dataset.csv",
				},
			],
		},
	]);
	const [error, setError] = useState<Error | null>(null);
	const [counter, setCounter] = useState(0);

	const simulations = useRef<Record<string, {}>>({});

	const addSim = useCallback(
		({ handle, schema }: { handle: SourceHandle; schema: JSONSchema7 }) => {
			setSources((sources) => [...sources, { handle, schema }]);
			const id = handle.id();

			const fields = [...schema.get("properties").entries()].map(
				([name, field]) => ({
					id: `${id}-${name}`,
					position: { x: 100, y: 80 },
					data: { label: name },
					type: "default",
					parentId: id,
					extent: "parent",
				}),
			);

			const newNodes = [
				{
					id,
					position: { x: 0, y: 0 },
					data: { label: id },
					width: 380,
					height: 200,
					type: "default",
				},
				...fields,
			];

			const minIndex = nodes.length;
			const simNodes = fields.map((n, id) => ({
				index: minIndex + id,
				id: n.id,
				x: 0,
				y: 0,
			}));

			// Custom force to put all nodes in a box
			function boxingForce() {
				for (const node of simNodes) {
					// Of the positions exceed the box, set them to the boundary position.
					// You may want to include your nodes width to not overlap with the box.
					node.x = Math.max(-0, Math.min(380, node.x));
					node.y = Math.max(-0, Math.min(200, node.y));
				}
			}

			// make sure there are nodes for all the inputs
			setNodes((nodes) => [...nodes, ...newNodes]);

			const simulation = forceSimulation(simNodes)
				.force("charge", forceManyBody())
				.force("boundaries", boxingForce)
				.on("tick", () => {
					setNodes((nodes) => {
						const updatedNodes = [...nodes];
						for (const node of simNodes) {
							if (updatedNodes?.[node.index + 1]) {
								updatedNodes[node.index].position.x = node.x;
								updatedNodes[node.index].position.y = node.y;
							}
						}
						return updatedNodes;
					});
				})
				.on("end", (event) => {
					console.log("END", event);
				});

			simulations.current[id] = { simulation, simNodes };
		},
		[setNodes],
	);

	useEffect(() => {
		try {
			console.log(sources);
			const resp = execute_query_json(
				sources.map((s) => s.handle),
				{
					type: "dsl",
					destination_schema: destinationFields,
					mappings,
					primary_keys: {
						keys: ["customer_id"],
					},
				},
			);
			console.log(resp);
			setError(null);
		} catch (e) {
			console.error(e);
			setError(e);
		}
	}, [sources, mappings, destinationFields, counter]);

	const onConnect = useCallback(
		(params: Connection) => setEdges((eds) => addEdge(params, eds)),
		[setEdges],
	);

	const onDragOver = useCallback((event: React.DragEvent) => {
		event.preventDefault();
		event.dataTransfer.dropEffect = "move";
	}, []);

	const onDrop = useCallback(
		async (event: React.DragEvent) => {
			event.preventDefault();

			// if it is a file, we need to load that file into a source and get its schema
			if (
				event.dataTransfer.items[0].kind === "file" &&
				event.dataTransfer.items[0].type.startsWith("text/csv")
			) {
				const data = event.dataTransfer.items[0].getAsFile();
				if (!data) return;
				const blob = await data.bytes();

				const source = new SourceHandle({
					Blob: {
						format: "csv",
						id: data.name,
						blob,
					},
				});

				const schema = source.schema();

				addSim({ handle: source, schema });

				return;
			}

			const reactFlowBounds = (event.target as Element).getBoundingClientRect();
			const type = event.dataTransfer.getData("application/reactflow");

			if (!type) return;

			const nodeData = JSON.parse(type);
			const position = {
				x: event.clientX - reactFlowBounds.left,
				y: event.clientY - reactFlowBounds.top,
			};

			const newNode: Node = {
				id: `${nodeData.type}-${Date.now()}`,
				type: nodeData.type,
				position,
				data: {
					label: nodeData.type,
					...nodeData.data,
					[nodeData.type === "source"
						? "source"
						: nodeData.type === "mapping"
							? "mapping"
							: "field"]: nodeData.data,
				},
			};

			setNodes((nds) => nds.concat(newNode));
		},
		[setNodes, addSim],
	);

	return (
		<div style={{ width: "100vw", height: "100vh", display: "flex" }}>
			{error && (
				<div
					style={{
						position: "absolute",
						top: "0",
						left: "0",
						right: "0",
						background: "red",
						color: "white",
						padding: "8px",
					}}
				>
					{error?.message ?? error ?? "Unknown error"}
				</div>
			)}
			<ReactFlow
				nodes={nodes}
				edges={edges}
				onNodesChange={(changes) => {
					onNodesChange(changes);
					for (const change of changes) {
						if (change.type !== "position") {
							continue;
						}

						for (const sim of Object.values(simulations.current)) {
							for (const node of sim.simNodes) {
								if (node.id === change.id) {
									if (change.position) {
										node.x = change.position.x;
										node.y = change.position.y;
									}
								}
							}
						}
					}
				}}
				onEdgesChange={onEdgesChange}
				onConnect={onConnect}
				onDrop={onDrop}
				onDragOver={onDragOver}
				nodeTypes={nodeTypes}
				fitView
			>
				<Controls />
				<MiniMap />
				<Background variant={BackgroundVariant.Dots} gap={12} size={1} />
			</ReactFlow>
		</div>
	);
}
