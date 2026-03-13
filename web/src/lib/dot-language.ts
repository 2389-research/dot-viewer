// ABOUTME: DOT language definition for CodeMirror 6 syntax highlighting.
// ABOUTME: Matches the macOS app's color scheme (purple keywords, green strings, etc).

import { StreamLanguage, type StreamParser } from '@codemirror/language';
import { tags } from '@lezer/highlight';

const dotParser: StreamParser<{ inBlockComment: boolean }> = {
	startState() {
		return { inBlockComment: false };
	},

	token(stream, state) {
		// Continue block comment
		if (state.inBlockComment) {
			const end = stream.match(/.*?\*\//, false);
			if (end) {
				stream.match(/.*?\*\//);
				state.inBlockComment = false;
			} else {
				stream.skipToEnd();
			}
			return 'comment';
		}

		// Skip whitespace
		if (stream.eatSpace()) return null;

		// Line comment
		if (stream.match('//')) {
			stream.skipToEnd();
			return 'lineComment';
		}

		// Block comment start
		if (stream.match('/*')) {
			state.inBlockComment = true;
			const end = stream.match(/.*?\*\//, false);
			if (end) {
				stream.match(/.*?\*\//);
				state.inBlockComment = false;
			} else {
				stream.skipToEnd();
			}
			return 'blockComment';
		}

		// Strings (double-quoted)
		if (stream.match(/"(?:[^"\\]|\\.)*"/)) {
			return 'string';
		}

		// HTML strings (angle-bracket quoted, simplified)
		if (stream.peek() === '<') {
			let depth = 0;
			while (!stream.eol()) {
				const ch = stream.next();
				if (ch === '<') depth++;
				if (ch === '>') {
					depth--;
					if (depth === 0) break;
				}
			}
			return 'string';
		}

		// Arrow operators
		if (stream.match('->') || stream.match('--')) {
			return 'punctuation';
		}

		// Braces, brackets, semicolons
		if (stream.match(/^[{}[\];,]/)) {
			return 'punctuation';
		}

		// Equals sign (attribute separator)
		if (stream.eat('=')) {
			return 'operator';
		}

		// Keywords and identifiers
		if (stream.match(/^[a-zA-Z_]\w*/)) {
			const word = stream.current();
			const keywords = [
				'digraph',
				'graph',
				'subgraph',
				'node',
				'edge',
				'strict',
			];
			if (keywords.includes(word)) {
				return 'keyword';
			}
			return 'variableName';
		}

		// Numbers
		if (stream.match(/^-?\d+\.?\d*/)) {
			return 'number';
		}

		// Skip unknown characters
		stream.next();
		return null;
	},
};

export const dotLanguage = StreamLanguage.define(dotParser);

// Theme matching the macOS app's syntax colors
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';

export const dotHighlightStyle = syntaxHighlighting(
	HighlightStyle.define([
		{ tag: tags.keyword, color: '#AF52DE' }, // systemPurple
		{ tag: tags.string, color: '#34C759' }, // systemGreen
		{ tag: tags.lineComment, color: '#8E8E93' }, // systemGray
		{ tag: tags.blockComment, color: '#8E8E93' },
		{ tag: tags.comment, color: '#8E8E93' },
		{ tag: tags.punctuation, color: '#FF9500' }, // systemOrange (arrows)
		{ tag: tags.variableName, color: '#007AFF' }, // systemBlue (identifiers/attributes)
		{ tag: tags.number, color: '#FF3B30' }, // systemRed
		{ tag: tags.operator, color: '#8E8E93' },
	]),
);
