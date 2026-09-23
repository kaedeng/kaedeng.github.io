import type { Metadata } from "next";
import type { ReactNode } from "react";
import { ResumeGate } from "@/components/ResumeGate";

export const metadata: Metadata = {
  title: "Kaelem Deng · Résumé",
};

type Entry = {
  title: string;
  org?: string;
  href?: string;
  place?: string;
  when?: string;
  points?: string[];
  tags?: string[];
};

const CONTACT = [
  { label: "hi@kae.codes", href: "mailto:hi@kae.codes" },
  { label: "LinkedIn", href: "https://linkedin.com/in/kaelem-deng" },
  { label: "GitHub", href: "https://github.com/kaedeng" },
];

const EDUCATION: Entry[] = [
  {
    title: "Accelerated Master’s Degree in Computer Science",
    place: "Colorado School of Mines",
    when: "May 2027",
  },
  {
    title: "Bachelor of Science in Computer Science",
    place: "Colorado School of Mines · GPA 3.89/4.0",
    when: "May 2026",
  },
];

const EXPERIENCE: Entry[] = [
  {
    title: "Software Engineer Intern",
    org: "Microsoft",
    place: "Redmond, WA",
    when: "May – Aug 2026",
    points: [
      "Built a Microsoft Graph Explorer integration into an internal chained-request workflow tool in C#, letting engineers on 10+ partner teams construct, chain, and test Graph API requests against isolated environments instead of hand-writing each call.",
      "Connected Azure AI Foundry to the tool’s MCP server, deployed behind Azure Front Door on Container Apps, so internal Graph API users could author and debug multi-step workflows conversationally.",
      "Cut manual click-through testing from 7 minutes to 30 seconds by building one-click cloud sync for locally drafted workflows.",
    ],
    tags: [
      "C#",
      "Microsoft Graph",
      "Azure AI Foundry",
      "MCP",
      "Container Apps",
    ],
  },
  {
    title: "AI/ML Student Researcher",
    org: "Colorado School of Mines",
    place: "Golden, CO",
    when: "Sep 2025 – Present",
    points: [
      "Built an adaptive LLM reasoning benchmark in Python that generates procedural planning problems with provably optimal solutions, using network flow algorithms and linear programming to compute ground truth.",
      "Evaluated frontier LLMs against LP-optimal baselines, measuring how plan quality degrades as problem size and constraint density scale.",
    ],
    tags: ["Python", "LLMs", "Linear programming", "Network flow"],
  },
  {
    title: "Software Engineer Intern",
    org: "Lockheed Martin",
    place: "Littleton, CO",
    when: "May – Aug 2025",
    points: [
      "Independently designed and deployed a cloud-native development environment on Kubernetes with Dev Containers, bringing new-developer setup for a 15-person team down to under 3 minutes.",
      "Refactored a TypeScript data-processing path handling dense payloads, reducing per-batch latency from 0.5s to 0.1s and removing duplicated parsing logic that had been blocking further changes.",
    ],
    tags: ["Kubernetes", "Dev Containers", "TypeScript"],
  },
];

const PROJECTS: Entry[] = [
  {
    title: "Packtrain",
    org: "Student Grade Orchestrator",
    href: "https://github.com/CSCI128/packtrain",
    points: [
      "Designed and built the REST API behind Packtrain, a grading and course-workflow service used by 450+ instructors and 7,500+ students, with Spring Java, PostgreSQL, and React-TypeScript on AWS; built with four other students under mandatory review on every merge.",
      "Kept concurrent grade and assignment submissions consistent through a normalized schema with composite indexes and transactional writes, cutting query latency roughly 20x under concurrent load.",
    ],
    tags: ["Java", "Spring", "PostgreSQL", "React", "AWS"],
  },
  {
    title: "AI-Generated Semantic Relations Graph",
    href: "https://github.com/Shokubai/ai-rel-graph-mvp",
    points: [
      "Built an async document ingestion pipeline in Python with FastAPI, Celery, and PostgreSQL + pgvector, indexing 10K+ documents for semantic similarity search.",
      "Combined pgvector cosine similarity with PostgreSQL full-text search for hybrid retrieval, recovering exact-term matches that pure vector lookup consistently missed.",
    ],
    tags: ["Python", "FastAPI", "Celery", "pgvector"],
  },
  {
    title: "Zen Browser",
    org: "Open source contributor",
    href: "https://github.com/zen-browser/desktop",
    points: [
      "Diagnosed and patched a state handling error in Zen’s UI layer; PR merged upstream.",
    ],
  },
];

const SKILLS = [
  {
    label: "Languages",
    items: ["Java", "C#", "JavaScript", "TypeScript", "Python", "SQL"],
  },
  {
    label: "Technologies",
    items: [
      "Unix/Linux",
      "Git",
      "Docker",
      "Java Spring",
      "FastAPI",
      "PostgreSQL",
      "pgvector",
      "React",
      "Kubernetes",
      "AWS",
      "Azure",
      "CI/CD",
      "Multithreading",
      "Concurrency",
      "Distributed Systems",
      "HTML5/CSS",
      "MCP",
      "Claude Code",
      "Codex",
    ],
  },
];

function Tags({ items }: { items: string[] }) {
  return (
    <ul className="flex flex-wrap gap-2">
      {items.map((t) => (
        <li
          key={t}
          className="rounded-full border border-white/15 px-2.5 py-0.5 text-xs text-zinc-300"
        >
          {t}
        </li>
      ))}
    </ul>
  );
}

/** A dates column on the left, the entry on the right; stacked on phones. */
function Row({ side, children }: { side?: string; children: ReactNode }) {
  return (
    <li className="grid gap-x-6 gap-y-1 sm:grid-cols-[8rem_1fr]">
      <p className="pt-0.5 text-sm text-zinc-500 tabular-nums">{side}</p>
      <div>{children}</div>
    </li>
  );
}

function EntryRow({ e }: { e: Entry }) {
  const title = e.href ? (
    <a href={e.href} className="hover:underline">
      {e.title}&nbsp;↗
    </a>
  ) : (
    e.title
  );
  return (
    <Row side={e.when}>
      <h3 className="font-medium">
        {title}
        {e.org && <span className="text-zinc-400"> · {e.org}</span>}
      </h3>
      {e.place && <p className="text-sm text-zinc-400">{e.place}</p>}
      {e.points && (
        <ul className="mt-3 list-disc space-y-2 pl-5 text-base text-zinc-300 marker:text-zinc-600">
          {e.points.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      )}
      {e.tags && (
        <div className="mt-4">
          <Tags items={e.tags} />
        </div>
      )}
    </Row>
  );
}

function Section({ title, entries }: { title: string; entries: Entry[] }) {
  return (
    <section className="mt-16">
      <h2 className="text-sm font-medium text-zinc-400">{title}</h2>
      <ul className="mt-6 space-y-10">
        {entries.map((e) => (
          <EntryRow key={e.title + e.when} e={e} />
        ))}
      </ul>
    </section>
  );
}

export default function Resume() {
  return (
    <ResumeGate>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        Kaelem Deng
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Software engineer and computer science master&apos;s student at Colorado
        School of Mines.
      </p>
      <ul className="mt-6 flex flex-wrap gap-x-6 gap-y-2 text-sm">
        {CONTACT.map((c) => (
          <li key={c.href}>
            <a href={c.href} className="text-zinc-300 hover:text-white">
              {c.label}
            </a>
          </li>
        ))}
      </ul>
      <Section title="Education" entries={EDUCATION} />
      <Section title="Experience" entries={EXPERIENCE} />
      <Section title="Projects" entries={PROJECTS} />
      <section className="mt-16">
        <h2 className="text-sm font-medium text-zinc-400">Skills</h2>
        <ul className="mt-6 space-y-6">
          {SKILLS.map((s) => (
            <Row key={s.label} side={s.label}>
              <Tags items={s.items} />
            </Row>
          ))}
        </ul>
      </section>
    </ResumeGate>
  );
}
