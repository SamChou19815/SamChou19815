import { allMetadata, permalinkFromMetadata } from "../../lib/metadata";
import { PROJECTS, SOCIAL, WORK } from "../../lib/site-data";

type FSFile = { type: "file"; content: string; mtime: string };
type FSDir = { type: "dir"; children: Record<string, FSNode>; mtime: string };
export type FSNode = FSFile | FSDir;

function file(content: string, mtime = "2024-01-01"): FSFile {
  return { type: "file", content: `${content.trim()}\n`, mtime };
}

function dir(children: Record<string, FSNode>, mtime = "2024-01-01"): FSDir {
  return { type: "dir", children, mtime };
}

function blogFiles(): Record<string, FSNode> {
  const children: Record<string, FSNode> = {
    "README.md": file(
      `# Developer Sam Blog

Use \`ls\` to browse posts. Read with \`cat <filename>\`.
`,
      "2024-05-27",
    ),
  };
  for (const meta of allMetadata) {
    const url = `https://developersam.com${permalinkFromMetadata(meta)}`;
    const mtime = `${meta.year}-${meta.month}-${meta.date}`;
    children[`${meta.year}-${meta.titleSlug}.md`] = file(url, mtime);
  }
  return children;
}

export const ROOT: FSDir = dir({
  home: dir({
    sam: dir(
      {
        "README.md": file(
          `# Developer Sam

Software engineer. Programming language enthusiast.

Currently working at Meta on the Flow type system, IDE, and performance.
Cornell University alumnus (B.S. + M.Eng in Computer Science).

## Quick Links

- GitHub: ${SOCIAL.github}
- Bluesky: ${SOCIAL.bluesky}
- LinkedIn: ${SOCIAL.linkedin}
- Resume: https://developersam.com/resume.pdf

## Explore

Try \`ls\` to see what's here, or \`cat\` any .md file to read it.
Run \`help\` for a list of available commands.
`,
          "2024-06-01",
        ),
        "links.md": file(
          `# Links

- Website: https://developersam.com
- GitHub: ${SOCIAL.github}
- Bluesky: ${SOCIAL.bluesky}
- LinkedIn: ${SOCIAL.linkedin}
- Resume: https://developersam.com/resume.pdf
- samlang: ${PROJECTS.samlang.website}
- Blog: https://developersam.com/blog
`,
          "2024-06-01",
        ),
        projects: dir(
          {
            "samlang.md": file(
              `# samlang

${PROJECTS.samlang.tagline}

A statically-typed functional programming language with a focus on
developer experience and correctness. Compiles to multiple targets
including LLVM, WebAssembly, and x86.

- Website: ${PROJECTS.samlang.website}
- GitHub: ${PROJECTS.samlang.github}
- Started: January 2019
- Written in: Rust (current), previously TypeScript, Kotlin

## Features

- Full Hindley-Milner type inference
- Pattern matching
- Bounded qualification (generics with constraints)
- Multiple compilation targets (LLVM, WASM, x86)
`,
              "2024-02-25",
            ),
            "samwise.md": file(
              `# Samwise

A task management app built at Cornell DTI.

Samwise helps Cornell students manage their academic tasks
with a clean, intuitive interface.

- GitHub: ${PROJECTS.samwise.github}
- Role: Frontend Developer
- Period: September 2018 - 2020
- Tech: React, TypeScript, Firebase
`,
              "2020-05-14",
            ),
            "courseplan.md": file(
              `# CoursePlan

A degree planning tool for Cornell students, built at Cornell DTI.

Helps students plan their four-year course schedule, check
requirements, and explore courses.

- Product: ${PROJECTS.coursePlan.product}
- GitHub: ${PROJECTS.coursePlan.github}
- Role: Developer
- Period: September 2021
`,
              "2022-01-02",
            ),
            "mini-react.md": file(
              `# mini-react

${PROJECTS.miniReact.tagline}

Demonstrates how React's core reconciliation algorithm and hooks
system work under the hood.

- GitHub: ${PROJECTS.miniReact.github}
- Demo: ${PROJECTS.miniReact.demo}
- Slides: https://developersam.com/build-simplified-react.pdf
- Created: May 2020
`,
              "2020-05-01",
            ),
            "ten.md": file(
              `# TEN

A strategy board game implementation.

- GitHub: ${PROJECTS.ten.github}
- Created: July 2017
- Written in: Go
`,
              "2017-07-01",
            ),
            "critter-world.md": file(
              `# Critter World

An implementation of the Critter World simulation for CS 2112 at Cornell.

Used as the course staff reference solution in Fall 2018 and Fall 2019.

- Course: CS 2112 (Object-Oriented Design and Data Structures - Honors)
- Completed: December 2017
`,
              "2017-12-01",
            ),
            "critter-compiler.md": file(
              `# Critter Compiler

A compiler that proves Critter World is Turing complete.

- GitHub: ${PROJECTS.critterCompiler.github}
- Created: August 2018
`,
              "2018-08-27",
            ),
            "SAMPL.md": file(
              `# SAMPL

${PROJECTS.sampl.tagline}

- GitHub: ${PROJECTS.sampl.github}
- Created: June 2018
`,
              "2018-06-15",
            ),
          },
          "2024-02-25",
        ),
        experience: dir(
          {
            "meta.md": file(
              `# Meta (Facebook) - Software Engineer

**Period:** February 2022 - Present
**Team:** Flow

${WORK.flow.tagline}

Flow is a static type checker for JavaScript.
- Website: ${WORK.flow.website}
- GitHub: ${WORK.flow.github}

## Internships at Facebook

### Summer 2021 - Flow Team
Implemented the extraction code action for Flow LSP.

### Summer 2020 - Feed Expressions Team
Worked on the Feed Expressions team.

### Summer 2019 - Pyre Team
Worked on Pyre, a performant type checker for Python.
- Website: ${WORK.pyre.website}
- GitHub: ${WORK.pyre.github}
`,
              "2024-01-01",
            ),
            "cornell-dti.md": file(
              `# Cornell Design & Tech Initiative (DTI)

## Roles

- **Developer Lead** (June 2019 - May 2021)
- **Technical Product Manager** - Samwise subteam (January 2019)
- **Software Developer** - Samwise (September 2018)
- **Software Developer** - CoursePlan (September 2021)

Website: ${WORK.cornellDTI.website}

## Projects

- Samwise: Task management app
- CoursePlan: Degree planning tool
`,
              "2022-01-02",
            ),
            "cornell.md": file(
              `# Cornell University

## M.Eng in Computer Science
Graduated: December 2021

## B.S. in Computer Science
Graduated: May 2021

Entered Cornell University in August 2017.
`,
              "2021-12-01",
            ),
          },
          "2024-01-01",
        ),
        blog: dir(blogFiles(), "2024-05-27"),
        ".profile": file(
          `# Sam's Profile
export USER=sam
export HOME=/home/sam
export PATH=/usr/bin:/bin
export EDITOR=vim
export LANG=en_US.UTF-8
alias ll='ls -l'
alias la='ls -la'
`,
          "2024-01-01",
        ),
      },
      "2024-06-01",
    ),
  }),
});

export function resolvePath(cwd: string, target: string): string {
  if (target.startsWith("/")) {
    return normalizePath(target);
  }
  return normalizePath(`${cwd}/${target}`);
}

function normalizePath(p: string): string {
  const parts = p.split("/").filter(Boolean);
  const resolved: string[] = [];
  for (const part of parts) {
    if (part === ".") continue;
    if (part === "..") {
      resolved.pop();
    } else {
      resolved.push(part);
    }
  }
  return `/${resolved.join("/")}`;
}

export function getNode(path: string): FSNode | null {
  if (path === "/") return ROOT;
  const parts = path.split("/").filter(Boolean);
  let current: FSNode = ROOT;
  for (const part of parts) {
    if (current.type !== "dir") return null;
    const child: FSNode | undefined = current.children[part];
    if (!child) return null;
    current = child;
  }
  return current;
}

export function getParentDir(path: string): FSDir | null {
  const parts = path.split("/").filter(Boolean);
  if (parts.length === 0) return null;
  parts.pop();
  const parentPath = `/${parts.join("/")}`;
  const node = getNode(parentPath);
  if (!node || node.type !== "dir") return null;
  return node;
}

export function basename(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? "/";
}
