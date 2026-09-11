/**
 * Typed integration for the local `issues` CLI.
 *
 * The tool deliberately invokes the executable with an argv array (never a shell
 * command), defaults to the current Pi session's `.scratch/issues.db`, and
 * serializes mutations against that database to avoid concurrent SQLite writes.
 */

import { StringEnum, Type } from "@earendil-works/pi-ai";
import {
	DEFAULT_MAX_BYTES,
	DEFAULT_MAX_LINES,
	formatSize,
	truncateHead,
	withFileMutationQueue,
	type ExtensionAPI,
} from "@earendil-works/pi-coding-agent";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const ISSUE_ACTIONS = [
	"create",
	"list",
	"get",
	"update",
	"close",
	"comment",
	"label_add",
	"label_remove",
	"dependency_add",
	"dependency_remove",
	"attach",
	"detach",
	"ready",
	"frontier",
	"blocked",
	"write_agent_instructions",
] as const;

const ISSUE_STATUSES = ["open", "in-progress", "blocked", "closed"] as const;

type IssueAction = (typeof ISSUE_ACTIONS)[number];
type IssueStatus = (typeof ISSUE_STATUSES)[number];

type IssueToolInput = {
	action: IssueAction;
	id?: number;
	title?: string;
	body?: string;
	bodyFile?: string;
	appendBody?: string;
	appendBodyFile?: string;
	status?: IssueStatus;
	label?: string;
	parentId?: number;
	parentIds?: number[];
	dependsOn?: number[];
	mapId?: number;
	openedAt?: string;
	clearOpenedAt?: boolean;
	resolvedBy?: string;
	clearResolvedBy?: boolean;
	comment?: string;
	outputPath?: string;
	databasePath?: string;
};

const issueParameters = Type.Object({
	action: StringEnum(ISSUE_ACTIONS, {
		description:
			"Operation: create, list, get, update, close, comment, label_add, label_remove, dependency_add, dependency_remove, attach, detach, ready, frontier, blocked, or write_agent_instructions.",
	}),
	id: Type.Optional(Type.Integer({ minimum: 1, description: "Issue ID for operations that target one issue." })),
	title: Type.Optional(Type.String({ description: "Title for create or update." })),
	body: Type.Optional(Type.String({ description: "Inline issue body for create/update/comment. May be empty when intentionally clearing an issue body." })),
	bodyFile: Type.Optional(Type.String({ description: "Path to a file used as the issue body. Cannot be combined with body." })),
	appendBody: Type.Optional(Type.String({ description: "Text to append to an issue body during update." })),
	appendBodyFile: Type.Optional(Type.String({ description: "Path to text appended to an issue body during update." })),
	status: Type.Optional(StringEnum(ISSUE_STATUSES, { description: "Issue status for list filtering or update." })),
	label: Type.Optional(Type.String({ description: "Label to filter by or add/remove." })),
	parentId: Type.Optional(Type.Integer({ minimum: 1, description: "Single parent issue ID used to filter list results." })),
	parentIds: Type.Optional(Type.Array(Type.Integer({ minimum: 1 }), { minItems: 1, description: "Parent issue IDs for attach or detach." })),
	dependsOn: Type.Optional(Type.Array(Type.Integer({ minimum: 1 }), { minItems: 1, description: "Blocking dependency issue IDs for dependency_add or dependency_remove." })),
	mapId: Type.Optional(Type.Integer({ minimum: 1, description: "Parent map ID used to restrict frontier results." })),
	openedAt: Type.Optional(Type.String({ description: "Git commit recorded as the issue opening anchor." })),
	clearOpenedAt: Type.Optional(Type.Boolean({ description: "Clear the opening commit anchor during update." })),
	resolvedBy: Type.Optional(Type.String({ description: "Git commit recorded as the resolving anchor." })),
	clearResolvedBy: Type.Optional(Type.Boolean({ description: "Clear the resolving commit anchor during update." })),
	comment: Type.Optional(Type.String({ description: "Resolution comment for close." })),
	outputPath: Type.Optional(Type.String({ description: "Output path for write_agent_instructions (default: docs/agents/issue-tracker.md)." })),
	databasePath: Type.Optional(Type.String({ description: "Optional database path. Defaults to .scratch/issues.db relative to the Pi session working directory." })),
});

const MUTATING_ACTIONS = new Set<IssueAction>([
	"create",
	"update",
	"close",
	"comment",
	"label_add",
	"label_remove",
	"dependency_add",
	"dependency_remove",
	"attach",
	"detach",
	"write_agent_instructions",
]);

function requireText(value: string | undefined, field: string): string {
	if (typeof value !== "string" || value.trim().length === 0) {
		throw new Error(`${field} is required and cannot be empty.`);
	}
	return value;
}

function requireIssueId(value: number | undefined, field = "id"): string {
	if (!Number.isSafeInteger(value) || value < 1) {
		throw new Error(`${field} is required and must be a positive integer.`);
	}
	return String(value);
}

function requireIssueIds(value: number[] | undefined, field: string): string[] {
	if (!Array.isArray(value) || value.length === 0) {
		throw new Error(`${field} is required and must contain at least one positive issue ID.`);
	}
	return value.map((id, index) => requireIssueId(id, `${field}[${index}]`));
}

function assertMutuallyExclusive(
	input: IssueToolInput,
	left: keyof IssueToolInput,
	right: keyof IssueToolInput,
): void {
	if (input[left] !== undefined && input[right] !== undefined) {
		throw new Error(`${String(left)} and ${String(right)} cannot be used together.`);
	}
}

function appendTextOption(args: string[], flag: string, value: string | undefined): void {
	if (value !== undefined) {
		args.push(flag, value);
	}
}

function appendRequiredTextOption(
	args: string[],
	flag: string,
	value: string | undefined,
	field: string,
): void {
	if (value !== undefined) {
		args.push(flag, requireText(value, field));
	}
}

function buildIssueArgs(input: IssueToolInput): string[] {
	const args: string[] = [];
	if (input.databasePath !== undefined) {
		args.push("--db", requireText(input.databasePath, "databasePath"));
	}

	switch (input.action) {
		case "create": {
			assertMutuallyExclusive(input, "body", "bodyFile");
			args.push("create", "--title", requireText(input.title, "title"));
			appendTextOption(args, "--body", input.body);
			appendRequiredTextOption(args, "--body-file", input.bodyFile, "bodyFile");
			appendRequiredTextOption(args, "--opened-at", input.openedAt, "openedAt");
			return args;
		}

		case "list": {
			args.push("list");
			if (input.status !== undefined) args.push("--status", input.status);
			appendRequiredTextOption(args, "--label", input.label, "label");
			if (input.parentId !== undefined) args.push("--parent", requireIssueId(input.parentId, "parentId"));
			return args;
		}

		case "get":
			return [...args, "get", requireIssueId(input.id)];

		case "update": {
			assertMutuallyExclusive(input, "body", "bodyFile");
			assertMutuallyExclusive(input, "appendBody", "appendBodyFile");
			if (input.openedAt !== undefined && input.clearOpenedAt) {
				throw new Error("openedAt and clearOpenedAt cannot be used together.");
			}
			if (input.resolvedBy !== undefined && input.clearResolvedBy) {
				throw new Error("resolvedBy and clearResolvedBy cannot be used together.");
			}

			const hasReplaceBody = input.body !== undefined || input.bodyFile !== undefined;
			const hasAppendBody = input.appendBody !== undefined || input.appendBodyFile !== undefined;
			if (hasReplaceBody && hasAppendBody) {
				throw new Error("body/bodyFile and appendBody/appendBodyFile cannot be used together.");
			}

			const hasNonStatusUpdate =
				input.title !== undefined ||
				hasReplaceBody ||
				hasAppendBody ||
				input.openedAt !== undefined ||
				input.clearOpenedAt === true ||
				input.resolvedBy !== undefined ||
				input.clearResolvedBy === true;
			if (input.status !== undefined && hasNonStatusUpdate) {
				throw new Error("status cannot be combined with title, body, append-body, or commit-anchor edits.");
			}
			if (input.status === undefined && !hasNonStatusUpdate) {
				throw new Error("update requires at least one field to change.");
			}

			args.push("update", requireIssueId(input.id));
			if (input.status !== undefined) args.push("--status", input.status);
			appendRequiredTextOption(args, "--title", input.title, "title");
			appendTextOption(args, "--body", input.body);
			appendRequiredTextOption(args, "--body-file", input.bodyFile, "bodyFile");
			appendTextOption(args, "--append-body", input.appendBody);
			appendRequiredTextOption(args, "--append-body-file", input.appendBodyFile, "appendBodyFile");
			appendRequiredTextOption(args, "--opened-at", input.openedAt, "openedAt");
			if (input.clearOpenedAt) args.push("--clear-opened-at");
			appendRequiredTextOption(args, "--resolved-by", input.resolvedBy, "resolvedBy");
			if (input.clearResolvedBy) args.push("--clear-resolved-by");
			return args;
		}

		case "close": {
			args.push("close", requireIssueId(input.id));
			appendTextOption(args, "--comment", input.comment);
			appendRequiredTextOption(args, "--resolved-by", input.resolvedBy, "resolvedBy");
			return args;
		}

		case "comment": {
			assertMutuallyExclusive(input, "body", "bodyFile");
			if (input.body === undefined && input.bodyFile === undefined) {
				throw new Error("comment requires body or bodyFile.");
			}
			args.push("comment", requireIssueId(input.id));
			appendTextOption(args, "--body", input.body);
			appendRequiredTextOption(args, "--body-file", input.bodyFile, "bodyFile");
			return args;
		}

		case "label_add":
			return [...args, "label", requireIssueId(input.id), "add", requireText(input.label, "label")];

		case "label_remove":
			return [...args, "label", requireIssueId(input.id), "remove", requireText(input.label, "label")];

		case "dependency_add": {
			args.push("depends", "add", requireIssueId(input.id));
			for (const dependencyId of requireIssueIds(input.dependsOn, "dependsOn")) {
				args.push("--on", dependencyId);
			}
			return args;
		}

		case "dependency_remove": {
			args.push("depends", "remove", requireIssueId(input.id));
			for (const dependencyId of requireIssueIds(input.dependsOn, "dependsOn")) {
				args.push("--on", dependencyId);
			}
			return args;
		}

		case "attach": {
			args.push("attach", requireIssueId(input.id));
			for (const parentId of requireIssueIds(input.parentIds, "parentIds")) {
				args.push("--parent", parentId);
			}
			return args;
		}

		case "detach": {
			args.push("detach", requireIssueId(input.id));
			for (const parentId of requireIssueIds(input.parentIds, "parentIds")) {
				args.push("--parent", parentId);
			}
			return args;
		}

		case "ready":
			return [...args, "ready"];

		case "frontier": {
			args.push("frontier");
			appendRequiredTextOption(args, "--label", input.label, "label");
			if (input.mapId !== undefined) args.push("--map", requireIssueId(input.mapId, "mapId"));
			return args;
		}

		case "blocked": {
			args.push("blocked");
			appendRequiredTextOption(args, "--label", input.label, "label");
			return args;
		}

		case "write_agent_instructions": {
			args.push("agent-instructions");
			appendRequiredTextOption(args, "--output", input.outputPath, "outputPath");
			return args;
		}
	}
}

function mutationTarget(input: IssueToolInput, cwd: string): string {
	if (input.action === "write_agent_instructions") {
		return resolve(cwd, input.outputPath ?? "docs/agents/issue-tracker.md");
	}
	return resolve(cwd, input.databasePath ?? ".scratch/issues.db");
}

async function formatOutput(output: string): Promise<{
	text: string;
	truncated: boolean;
	fullOutputPath?: string;
}> {
	const truncation = truncateHead(output, {
		maxLines: DEFAULT_MAX_LINES,
		maxBytes: DEFAULT_MAX_BYTES,
	});
	if (!truncation.truncated) {
		return { text: truncation.content || "(issues returned no output)", truncated: false };
	}

	const tempDirectory = await mkdtemp(join(tmpdir(), "pi-issues-"));
	const fullOutputPath = join(tempDirectory, "output.txt");
	await withFileMutationQueue(fullOutputPath, async () => writeFile(fullOutputPath, output, "utf8"));

	let text = truncation.content || "(first output line exceeds the display limit)";
	text += `\n\n[Output truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines`;
	text += ` (${formatSize(truncation.outputBytes)} of ${formatSize(truncation.totalBytes)}).`;
	text += ` Full output saved to: ${fullOutputPath}]`;
	return { text, truncated: true, fullOutputPath };
}

function combineProcessOutput(stdout: string, stderr: string): string {
	const sections: string[] = [];
	if (stdout.trim()) sections.push(stdout.trimEnd());
	if (stderr.trim()) sections.push(`stderr:\n${stderr.trimEnd()}`);
	return sections.join("\n\n") || "(issues returned no output)";
}

export default function issuesExtension(pi: ExtensionAPI): void {
	pi.registerTool({
		name: "issues",
		label: "Issues",
		description: `Manage the current project's local SQLite issue tracker through the installed issues CLI. It defaults to .scratch/issues.db in the Pi session working directory and returns plain, byte-stable CLI output. Output is truncated to ${DEFAULT_MAX_LINES} lines or ${formatSize(DEFAULT_MAX_BYTES)}; full truncated output is saved to a temporary file.`,
		promptSnippet: "Create, inspect, organize, and resolve local project issues in .scratch/issues.db",
		promptGuidelines: [
			"Use issues instead of constructing shell commands when the user asks about the local issue tracker.",
			"Use manage_todo_list for short-lived execution steps in the current request; use issues for durable project work that should survive the session.",
			"Use issues mutations only when the user requests them or the current task has demonstrably changed the issue state; do not close issues speculatively.",
			"When working on an existing issue, claim it in-progress, track immediate steps in manage_todo_list, record durable findings as issue comments, and close only after acceptance criteria are satisfied.",
		],
		parameters: issueParameters,

		async execute(_toolCallId, rawParams, signal, onUpdate, ctx) {
			const input = rawParams as IssueToolInput;
			if (signal?.aborted) {
				return {
					content: [{ type: "text", text: "Cancelled before running issues." }],
					details: { action: input.action, cancelled: true },
				};
			}

			const args = buildIssueArgs(input);
			onUpdate?.({
				content: [{ type: "text", text: `Running issues ${input.action}...` }],
				details: { action: input.action },
			});

			const run = async () => pi.exec("issues", args, {
				cwd: ctx.cwd,
				signal,
				timeout: 15_000,
			});

			let result;
			try {
				result = MUTATING_ACTIONS.has(input.action)
					? await withFileMutationQueue(mutationTarget(input, ctx.cwd), run)
					: await run();
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				throw new Error(`Unable to run issues: ${message}`);
			}

			const displayed = await formatOutput(combineProcessOutput(result.stdout, result.stderr));
			if (result.code !== 0) {
				throw new Error(`issues ${input.action} failed with exit code ${result.code}.\n${displayed.text}`);
			}

			return {
				content: [{ type: "text", text: displayed.text }],
				details: {
					action: input.action,
					databasePath: input.databasePath ?? ".scratch/issues.db",
					exitCode: result.code,
					truncated: displayed.truncated,
					fullOutputPath: displayed.fullOutputPath,
				},
			};
		},
	});
}
