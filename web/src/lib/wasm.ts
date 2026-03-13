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

let parserModule: typeof import("dot-core-wasm");
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
		const [parser, graphvizModule] = await Promise.all([
			import("dot-core-wasm"),
			import("@hpcc-js/wasm-graphviz"),
		]);

		// Initialize the dot-core WASM parser
		await parser.default();
		const graphviz = await graphvizModule.Graphviz.load();
		parserModule = parser;
		graphvizInstance = graphviz;
	})().catch((error) => {
		initPromise = null;
		throw error;
	});

	return initPromise;
}

// --- Primitive API (mirrors UniFFI exports 1:1) ---

/**
 * Parse DOT source into a structured graph model.
 */
export async function parseDot(source: string): Promise<DotGraph> {
	await ensureInit();
	return parserModule.parseDot(source) as DotGraph;
}

/**
 * Find the statement containing the given character offset.
 */
export async function statementAt(
	source: string,
	offset: number,
): Promise<DotStatement | null> {
	await ensureInit();
	const result = parserModule.statementAt(source, offset);
	return result as DotStatement | null;
}

/**
 * Returns the node ID relevant to a given cursor offset within a statement.
 * For node definitions, always returns the node ID.
 * For edges, returns whichever node the cursor is closest to.
 */
export async function nodeIdAt(
	source: string,
	statementOffset: number,
	cursorOffset: number,
): Promise<string | undefined> {
	await ensureInit();
	return parserModule.nodeIdAt(source, statementOffset, cursorOffset);
}

/**
 * Find the first node definition for a given node ID, falling back to
 * any edge referencing it.
 */
export async function definitionForNode(
	source: string,
	nodeId: string,
): Promise<DotStatement | null> {
	await ensureInit();
	const result = parserModule.definitionForNode(source, nodeId);
	return result as DotStatement | null;
}

/**
 * Find the source range for the definition of a given node ID.
 */
export async function definitionRangeForNode(
	source: string,
	nodeId: string,
): Promise<SourceRange | undefined> {
	await ensureInit();
	const result: Uint32Array | undefined =
		parserModule.definitionRangeForNode(source, nodeId);
	if (!result) return undefined;
	return { location: result[0], length: result[1] };
}

// --- Convenience API (composed from primitives) ---

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
 * Convenience wrapper that combines statementAt + nodeIdAt.
 */
export async function nodeIdAtOffset(
	source: string,
	offset: number,
): Promise<string | undefined> {
	await ensureInit();
	const stmt = parserModule.statementAt(source, offset);
	if (!stmt || stmt === null) return undefined;
	const stmtObj = stmt as DotStatement;
	return parserModule.nodeIdAt(source, stmtObj.source_range.location, offset);
}
