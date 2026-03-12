// ABOUTME: Lazy-loading wrapper for the dot-core WASM parser and Graphviz renderer.
// ABOUTME: Initializes both WASM modules on first use and provides typed access to their APIs.

import type { Engine, Graphviz } from "@hpcc-js/wasm-graphviz";

// TypeScript interfaces matching the serde output from dot-core-wasm

export interface SourceRange {
	location: number;
	length: number;
}

export interface NodeDefinition {
	type: "NodeDefinition";
	id: string;
	source_range: SourceRange;
}

export interface Edge {
	type: "Edge";
	from: string;
	to: string;
	source_range: SourceRange;
	from_range: SourceRange;
	to_range: SourceRange;
}

export interface GraphAttribute {
	type: "GraphAttribute";
	source_range: SourceRange;
}

export type DotStatement = NodeDefinition | Edge | GraphAttribute;

export interface DotGraph {
	statements: DotStatement[];
}

// Lazy-initialized module references

let parserReady = false;
let graphvizInstance: Graphviz;

let initPromise: Promise<void> | null = null;

/**
 * Initializes both WASM modules on first call. Subsequent calls return
 * the same promise so initialization only happens once.
 */
async function ensureInit(): Promise<void> {
	if (initPromise) {
		return initPromise;
	}

	initPromise = (async () => {
		const [parserModule, graphvizModule] = await Promise.all([
			import("dot-core-wasm"),
			import("@hpcc-js/wasm-graphviz"),
		]);

		// Initialize the dot-core WASM parser
		await parserModule.default();
		parserReady = true;

		// Initialize the Graphviz renderer
		graphvizInstance = await graphvizModule.Graphviz.load();
	})();

	return initPromise;
}

/**
 * Parse DOT source into a structured graph model.
 */
export async function parseDot(source: string): Promise<DotGraph> {
	await ensureInit();
	const { parseDot: wasmParseDot } = await import("dot-core-wasm");
	return wasmParseDot(source) as DotGraph;
}

/**
 * Render DOT source to SVG using the Graphviz WASM engine.
 */
export async function renderDot(
	source: string,
	engine: Engine = "dot",
): Promise<string> {
	await ensureInit();
	return graphvizInstance.layout(source, "svg", engine);
}

/**
 * Returns the node ID at the given cursor offset in the DOT source,
 * or undefined if the offset is not within a node reference.
 */
export async function nodeIdAtOffset(
	source: string,
	offset: number,
): Promise<string | undefined> {
	await ensureInit();
	const { nodeIdAtOffset: wasmNodeIdAtOffset } = await import("dot-core-wasm");
	return wasmNodeIdAtOffset(source, offset);
}

/**
 * Returns the source offset of the definition for a given node ID,
 * or undefined if the node is not found.
 */
export async function definitionOffsetForNode(
	source: string,
	nodeId: string,
): Promise<number | undefined> {
	await ensureInit();
	const { definitionOffsetForNode: wasmDefinitionOffsetForNode } =
		await import("dot-core-wasm");
	return wasmDefinitionOffsetForNode(source, nodeId);
}
