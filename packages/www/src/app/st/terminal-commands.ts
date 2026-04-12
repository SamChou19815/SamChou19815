import { type FSNode, basename, getNode, resolvePath } from "./terminal-fs";

export type TerminalState = {
  cwd: string;
  history: OutputLine[];
  inputHistory: string[];
};

export type OutputLine =
  | { type: "input"; text: string }
  | { type: "output"; text: string }
  | { type: "error"; text: string };

function out(text: string): OutputLine {
  return { type: "output", text };
}

function err(text: string): OutputLine {
  return { type: "error", text };
}

function parseArgs(input: string): { cmd: string; args: string[]; flags: Set<string> } {
  const tokens = shellSplit(input.trim());
  const cmd = tokens[0] ?? "";
  const args: string[] = [];
  const flags = new Set<string>();
  for (let i = 1; i < tokens.length; i++) {
    const t = tokens[i];
    if (!t) continue;
    if (t.startsWith("-") && t.length > 1 && !t.startsWith("--")) {
      for (const c of t.slice(1)) flags.add(c);
    } else if (t.startsWith("--")) {
      flags.add(t.slice(2));
    } else {
      args.push(t);
    }
  }
  return { cmd, args, flags };
}

function shellSplit(s: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let inSingle = false;
  let inDouble = false;
  for (const c of s) {
    if (c === "'" && !inDouble) {
      inSingle = !inSingle;
    } else if (c === '"' && !inSingle) {
      inDouble = !inDouble;
    } else if (c === " " && !inSingle && !inDouble) {
      if (current) {
        tokens.push(current);
        current = "";
      }
    } else {
      current += c;
    }
  }
  if (current) tokens.push(current);
  return tokens;
}

function formatPermissions(node: FSNode): string {
  return node.type === "dir" ? "drwxr-xr-x" : "-rw-r--r--";
}

function formatSize(node: FSNode): string {
  if (node.type === "dir") return "4096";
  return String(node.content.length);
}

function formatLsEntry(name: string, node: FSNode, long: boolean): string {
  if (!long) {
    return node.type === "dir" ? `${name}/` : name;
  }
  const perms = formatPermissions(node);
  const size = formatSize(node).padStart(6);
  const date = node.mtime;
  const display = node.type === "dir" ? `${name}/` : name;
  return `${perms}  sam sam ${size} ${date} ${display}`;
}

function cmdLs(cwd: string, args: string[], flags: Set<string>): OutputLine[] {
  const showHidden = flags.has("a");
  const long = flags.has("l");
  const target = args[0] ? resolvePath(cwd, args[0]) : cwd;
  const node = getNode(target);
  if (!node) return [err(`ls: cannot access '${args[0]}': No such file or directory`)];
  if (node.type === "file") {
    return [out(formatLsEntry(basename(target), node, long))];
  }
  const entries = Object.entries(node.children)
    .filter(([name]) => showHidden || !name.startsWith("."))
    .sort(([a], [b]) => a.localeCompare(b));
  if (entries.length === 0) return [];
  if (long) {
    return [
      out(`total ${entries.length}`),
      ...entries.map(([name, n]) => out(formatLsEntry(name, n, true))),
    ];
  }
  return entries.map(([name, n]) => out(formatLsEntry(name, n, false)));
}

function cmdCd(cwd: string, args: string[]): { newCwd: string; output: OutputLine[] } {
  const target = args[0] ?? "/home/sam";
  const resolved = resolvePath(cwd, target);
  const node = getNode(resolved);
  if (!node) return { newCwd: cwd, output: [err(`cd: no such file or directory: ${target}`)] };
  if (node.type !== "dir") return { newCwd: cwd, output: [err(`cd: not a directory: ${target}`)] };
  return { newCwd: resolved, output: [] };
}

function cmdCat(cwd: string, args: string[], stdin: string | null): OutputLine[] {
  if (args.length === 0) {
    return stdin !== null ? [out(stdin)] : [err("cat: missing operand")];
  }
  const lines: OutputLine[] = [];
  for (const arg of args) {
    const resolved = resolvePath(cwd, arg);
    const node = getNode(resolved);
    if (!node) {
      lines.push(err(`cat: ${arg}: No such file or directory`));
    } else if (node.type === "dir") {
      lines.push(err(`cat: ${arg}: Is a directory`));
    } else {
      lines.push(out(node.content));
    }
  }
  return lines;
}

function cmdHead(
  cwd: string,
  args: string[],
  flags: Set<string>,
  stdin: string | null,
): OutputLine[] {
  const nIndex = args.findIndex((a) => /^\d+$/.test(a));
  let n = 10;
  let files = args;
  if (flags.has("n") && nIndex !== -1) {
    n = Number.parseInt(args[nIndex] ?? "", 10);
    files = args.filter((_, i) => i !== nIndex);
  }
  if (files.length === 0) {
    if (stdin !== null) return [out(stdin.split("\n").slice(0, n).join("\n"))];
    return [err("head: missing operand")];
  }
  const lines: OutputLine[] = [];
  for (const arg of files) {
    const resolved = resolvePath(cwd, arg);
    const node = getNode(resolved);
    if (!node) {
      lines.push(err(`head: ${arg}: No such file or directory`));
    } else if (node.type === "dir") {
      lines.push(err(`head: ${arg}: Is a directory`));
    } else {
      const fileLines = node.content.split("\n").slice(0, n);
      lines.push(out(fileLines.join("\n")));
    }
  }
  return lines;
}

function cmdTail(
  cwd: string,
  args: string[],
  flags: Set<string>,
  stdin: string | null,
): OutputLine[] {
  const nIndex = args.findIndex((a) => /^\d+$/.test(a));
  let n = 10;
  let files = args;
  if (flags.has("n") && nIndex !== -1) {
    n = Number.parseInt(args[nIndex] ?? "", 10);
    files = args.filter((_, i) => i !== nIndex);
  }
  if (files.length === 0) {
    if (stdin !== null) return [out(stdin.trimEnd().split("\n").slice(-n).join("\n"))];
    return [err("tail: missing operand")];
  }
  const lines: OutputLine[] = [];
  for (const arg of files) {
    const resolved = resolvePath(cwd, arg);
    const node = getNode(resolved);
    if (!node) {
      lines.push(err(`tail: ${arg}: No such file or directory`));
    } else if (node.type === "dir") {
      lines.push(err(`tail: ${arg}: Is a directory`));
    } else {
      const fileLines = node.content.trimEnd().split("\n");
      lines.push(out(fileLines.slice(-n).join("\n")));
    }
  }
  return lines;
}

function treeHelper(path: string, prefix: string): string[] {
  const node = getNode(path);
  if (!node || node.type !== "dir") return [];
  const entries = Object.entries(node.children)
    .filter(([name]) => !name.startsWith("."))
    .sort(([a], [b]) => a.localeCompare(b));
  const lines: string[] = [];
  entries.forEach(([name, child], i) => {
    const isLast = i === entries.length - 1;
    const connector = isLast ? "└── " : "├── ";
    const display = child.type === "dir" ? `${name}/` : name;
    lines.push(`${prefix}${connector}${display}`);
    if (child.type === "dir") {
      const newPrefix = prefix + (isLast ? "    " : "│   ");
      lines.push(...treeHelper(`${path === "/" ? "" : path}/${name}`, newPrefix));
    }
  });
  return lines;
}

function cmdTree(cwd: string, args: string[]): OutputLine[] {
  const target = args[0] ? resolvePath(cwd, args[0]) : cwd;
  const node = getNode(target);
  if (!node) return [err(`tree: '${args[0]}': No such file or directory`)];
  if (node.type !== "dir") return [out(basename(target))];
  const header = args[0] ?? ".";
  const lines = treeHelper(target, "");
  return [out(`${header}\n${lines.join("\n")}`)];
}

function cmdGrep(cwd: string, args: string[], stdin: string | null): OutputLine[] {
  if (args.length === 0) return [err("usage: grep PATTERN FILE...")];
  const pattern = args[0] ?? "";
  let regex: RegExp;
  try {
    regex = new RegExp(pattern, "i");
  } catch {
    return [err(`grep: invalid regex: ${pattern}`)];
  }
  if (args.length < 2) {
    if (stdin !== null) {
      return stdin
        .split("\n")
        .filter((line) => regex.test(line))
        .map((l) => out(l));
    }
    return [err("usage: grep PATTERN FILE...")];
  }
  const lines: OutputLine[] = [];
  for (const arg of args.slice(1)) {
    const resolved = resolvePath(cwd, arg);
    const node = getNode(resolved);
    if (!node) {
      lines.push(err(`grep: ${arg}: No such file or directory`));
    } else if (node.type === "dir") {
      lines.push(err(`grep: ${arg}: Is a directory`));
    } else {
      for (const line of node.content.split("\n")) {
        if (regex.test(line)) {
          lines.push(out(args.length > 2 ? `${arg}:${line}` : line));
        }
      }
    }
  }
  return lines;
}

function grepRecursive(path: string, regex: RegExp): OutputLine[] {
  const node = getNode(path);
  if (!node) return [];
  if (node.type === "file") {
    const lines: OutputLine[] = [];
    for (const line of node.content.split("\n")) {
      if (regex.test(line)) {
        lines.push(out(`${path}:${line}`));
      }
    }
    return lines;
  }
  const results: OutputLine[] = [];
  for (const [name, child] of Object.entries(node.children).sort(([a], [b]) =>
    a.localeCompare(b),
  )) {
    const childPath = `${path === "/" ? "" : path}/${name}`;
    if (child.type === "file") {
      for (const line of child.content.split("\n")) {
        if (regex.test(line)) {
          results.push(out(`${childPath}:${line}`));
        }
      }
    } else {
      results.push(...grepRecursive(childPath, regex));
    }
  }
  return results;
}

function cmdGrepFull(
  cwd: string,
  args: string[],
  flags: Set<string>,
  stdin: string | null,
): OutputLine[] {
  if (flags.has("r") || flags.has("R")) {
    if (args.length < 1) return [err("usage: grep -r PATTERN [DIR]")];
    const pattern = args[0] ?? "";
    let regex: RegExp;
    try {
      regex = new RegExp(pattern, "i");
    } catch {
      return [err(`grep: invalid regex: ${pattern}`)];
    }
    const target = args[1] ? resolvePath(cwd, args[1]) : cwd;
    return grepRecursive(target, regex);
  }
  return cmdGrep(cwd, args, stdin);
}

function cmdFind(cwd: string, args: string[]): OutputLine[] {
  const target = args[0] ? resolvePath(cwd, args[0]) : cwd;
  const nameIdx = args.indexOf("-name");
  const pattern = (nameIdx !== -1 ? args[nameIdx + 1] : undefined) ?? null;

  const results: string[] = [];

  function walk(path: string) {
    const node = getNode(path);
    if (!node) return;
    const name = basename(path);
    if (!pattern || matchGlob(name, pattern)) {
      results.push(path === target ? "." : `.${path.slice(target.length)}`);
    }
    if (node.type === "dir") {
      for (const childName of Object.keys(node.children).sort()) {
        walk(`${path === "/" ? "" : path}/${childName}`);
      }
    }
  }

  walk(target);
  return results.map((r) => out(r));
}

function matchGlob(name: string, pattern: string): boolean {
  const regex = pattern
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*")
    .replace(/\?/g, ".");
  return new RegExp(`^${regex}$`).test(name);
}

function formatWc(content: string, flags: Set<string>, label?: string): string {
  const lOnly = flags.has("l");
  const lineCount = content.split("\n").length - 1;
  if (lOnly) {
    return label ? `  ${String(lineCount).padStart(4)} ${label}` : String(lineCount);
  }
  const wordCount = content.split(/\s+/).filter(Boolean).length;
  const charCount = content.length;
  const counts = `  ${String(lineCount).padStart(4)} ${String(wordCount).padStart(4)} ${String(charCount).padStart(4)}`;
  return label ? `${counts} ${label}` : counts;
}

function cmdWc(
  cwd: string,
  args: string[],
  flags: Set<string>,
  stdin: string | null,
): OutputLine[] {
  if (args.length === 0) {
    if (stdin !== null) return [out(formatWc(stdin, flags))];
    return [err("wc: missing operand")];
  }
  const lines: OutputLine[] = [];
  for (const arg of args) {
    const resolved = resolvePath(cwd, arg);
    const node = getNode(resolved);
    if (!node) {
      lines.push(err(`wc: ${arg}: No such file or directory`));
    } else if (node.type === "dir") {
      lines.push(err(`wc: ${arg}: Is a directory`));
    } else {
      lines.push(out(formatWc(node.content, flags, arg)));
    }
  }
  return lines;
}

function cmdXdgOpen(cwd: string, args: string[]): OutputLine[] {
  if (args.length === 0) return [err("open: missing operand")];
  const resolved = resolvePath(cwd, args[0] ?? "");
  const node = getNode(resolved);
  if (!node) return [err(`open: ${args[0]}: No such file or directory`)];
  if (node.type !== "file") return [err(`open: ${args[0]}: Is a directory`)];
  const urlMatch = node.content.match(/https?:\/\/[^\s)]+/);
  if (urlMatch) {
    return [out(`Opening ${urlMatch[0]} ...`), { type: "output", text: `\x1BOPEN:${urlMatch[0]}` }];
  }
  return [out(`No URL found in ${args[0]}`)];
}

const HELP_TEXT = `Available commands:

  ls [path]           List directory contents (-l, -a)
  cd [path]           Change directory
  cat <file>          Display file contents
  head [-n N] <file>  Display first N lines (default 10)
  tail [-n N] <file>  Display last N lines (default 10)
  pwd                 Print working directory
  tree [path]         Display directory tree
  find [path] [-name pattern]
                      Find files by name
  grep [-r] <pattern> <file|dir>
                      Search file contents
  wc <file>           Word, line, and character counts
  echo <text>         Display text
  whoami              Print current user
  date                Print current date
  open <file>         Open first URL found in file
  uname [-a]          Print system information
  env                 Print environment variables
  clear               Clear the terminal
  history             Show command history
  help                Show this help message

Navigation tips:
  Start with "cat README.md" to learn about this site.
  Use "ls" and "cd" to browse projects, blog, and experience.
  Tab completion is supported.`;

function executeSingle(
  input: string,
  state: TerminalState,
  stdin: string | null,
): { output: OutputLine[]; newCwd: string } {
  const trimmed = input.trim();
  if (!trimmed) return { output: [], newCwd: state.cwd };

  const { cmd, args, flags } = parseArgs(trimmed);

  switch (cmd) {
    case "ls":
      return { output: cmdLs(state.cwd, args, flags), newCwd: state.cwd };
    case "cd": {
      const result = cmdCd(state.cwd, args);
      return { output: result.output, newCwd: result.newCwd };
    }
    case "cat":
      return { output: cmdCat(state.cwd, args, stdin), newCwd: state.cwd };
    case "head":
      return { output: cmdHead(state.cwd, args, flags, stdin), newCwd: state.cwd };
    case "tail":
      return { output: cmdTail(state.cwd, args, flags, stdin), newCwd: state.cwd };
    case "pwd":
      return { output: [out(state.cwd)], newCwd: state.cwd };
    case "tree":
      return { output: cmdTree(state.cwd, args), newCwd: state.cwd };
    case "grep":
      return { output: cmdGrepFull(state.cwd, args, flags, stdin), newCwd: state.cwd };
    case "find":
      return { output: cmdFind(state.cwd, args), newCwd: state.cwd };
    case "wc":
      return { output: cmdWc(state.cwd, args, flags, stdin), newCwd: state.cwd };
    case "echo":
      return { output: [out(args.join(" "))], newCwd: state.cwd };
    case "whoami":
      return { output: [out("sam")], newCwd: state.cwd };
    case "hostname":
      return { output: [out("developersam.com")], newCwd: state.cwd };
    case "date":
      return { output: [out(new Date().toString())], newCwd: state.cwd };
    case "uname":
      if (flags.has("a")) {
        return {
          output: [out("SamOS 1.0.0 developersam.com x86_64 TypeScript/React")],
          newCwd: state.cwd,
        };
      }
      return { output: [out("SamOS")], newCwd: state.cwd };
    case "env":
      return {
        output: [
          out("USER=sam"),
          out("HOME=/home/sam"),
          out("PATH=/usr/bin:/bin"),
          out("SHELL=/bin/bash"),
          out("EDITOR=vim"),
          out("LANG=en_US.UTF-8"),
          out("TERM=xterm-256color"),
          out("HOSTNAME=developersam.com"),
        ],
        newCwd: state.cwd,
      };
    case "open":
    case "xdg-open":
      return { output: cmdXdgOpen(state.cwd, args), newCwd: state.cwd };
    case "clear":
      return { output: [{ type: "output", text: "\x1BCLEAR" }], newCwd: state.cwd };
    case "history":
      return {
        output: state.inputHistory.map((h, i) => out(`  ${String(i + 1).padStart(4)}  ${h}`)),
        newCwd: state.cwd,
      };
    case "help":
    case "man":
      return { output: [out(HELP_TEXT)], newCwd: state.cwd };
    case "exit":
      return {
        output: [out("Goodbye! (This is a web terminal, nowhere to exit to 😄)")],
        newCwd: state.cwd,
      };
    case "vim":
    case "nano":
    case "vi":
      return { output: [err(`${cmd}: this is a read-only terminal`)], newCwd: state.cwd };
    case "rm":
    case "rmdir":
    case "mkdir":
    case "touch":
    case "cp":
    case "mv":
      return { output: [err(`${cmd}: read-only file system`)], newCwd: state.cwd };
    case "sudo":
      return {
        output: [err("sam is not in the sudoers file. This incident will be reported.")],
        newCwd: state.cwd,
      };
    case "neofetch":
      return {
        output: [
          out("        .--.         sam@sam-terminal"),
          out("       |o_o |        --------------------"),
          out("       |:_/ |        OS: SamOS 1.0.0"),
          out("      //   \\ \\       Host: developersam.com"),
          out("     (|     | )      Kernel: React 18 + Next.js"),
          out(`    /'\\_   _/\`\\      Shell: sam-terminal`),
          out("    \\___)=(___/      Languages: TypeScript, Rust, OCaml"),
          out("                     Projects: samlang, samwise, mini-react"),
          out("                     Education: Cornell (BS + MEng CS)"),
          out("                     Work: Meta (Flow team)"),
        ],
        newCwd: state.cwd,
      };
    default:
      return { output: [err(`${cmd}: command not found`)], newCwd: state.cwd };
  }
}

export function executeCommand(
  input: string,
  state: TerminalState,
): { output: OutputLine[]; newCwd: string } {
  const trimmed = input.trim();
  if (!trimmed) return { output: [], newCwd: state.cwd };

  if (!trimmed.includes(" | ")) {
    return executeSingle(trimmed, state, null);
  }

  const segments = trimmed.split(" | ");
  let cwd = state.cwd;
  let stdin: string | null = null;
  const errors: OutputLine[] = [];

  for (const segment of segments) {
    const result = executeSingle(segment.trim(), { ...state, cwd }, stdin);
    cwd = result.newCwd;
    for (const line of result.output) {
      if (line.type === "error") errors.push(line);
    }
    stdin = result.output
      .filter((l) => l.type === "output")
      .map((l) => l.text)
      .join("\n");
  }

  const finalOutput = stdin ? stdin.split("\n").map((l) => out(l)) : [];
  return { output: [...errors, ...finalOutput], newCwd: cwd };
}

export function getCompletions(partial: string, cwd: string): string[] {
  const tokens = partial.split(/\s+/);
  const commands = [
    "ls",
    "cd",
    "cat",
    "head",
    "tail",
    "pwd",
    "tree",
    "find",
    "grep",
    "wc",
    "echo",
    "whoami",
    "hostname",
    "date",
    "uname",
    "env",
    "open",
    "clear",
    "history",
    "help",
    "neofetch",
    "exit",
  ];

  // Complete command name
  if (tokens.length <= 1) {
    const prefix = tokens[0] ?? "";
    return commands.filter((c) => c.startsWith(prefix));
  }

  // Complete file/directory path
  const lastToken = tokens[tokens.length - 1] ?? "";
  const slashIdx = lastToken.lastIndexOf("/");
  let dirPath: string;
  let prefix: string;

  if (slashIdx !== -1) {
    const dirPart = lastToken.slice(0, slashIdx) || "/";
    dirPath = resolvePath(cwd, dirPart);
    prefix = lastToken.slice(slashIdx + 1);
  } else {
    dirPath = cwd;
    prefix = lastToken;
  }

  const node = getNode(dirPath);
  if (!node || node.type !== "dir") return [];

  return Object.entries(node.children)
    .filter(([name]) => name.startsWith(prefix))
    .map(([name, child]) => {
      const basePart = slashIdx !== -1 ? `${lastToken.slice(0, slashIdx + 1)}${name}` : name;
      return child.type === "dir" ? `${basePart}/` : basePart;
    });
}
