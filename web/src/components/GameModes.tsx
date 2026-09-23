"use client";

import {
  Suspense,
  useEffect,
  useState,
  useSyncExternalStore,
  type FormEvent,
} from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { generatePuzzle, type Level, type Puzzle } from "patches-board";
import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";
import { Solution } from "@/components/Solution";
import { Tutorial } from "@/components/Tutorial";
import { seedParam, withSeed } from "@/lib/seed";
import { useDailyPuzzle, useToday } from "@/lib/useDaily";

type Mode = "daily" | "random";

const MODES: [Mode, string][] = [
  ["daily", "Daily"],
  ["random", "Random"],
];

const LEVELS: [Level, string][] = [
  ["easy", "Easy"],
  ["medium", "Medium"],
  ["hard", "Hard"],
];

/** How long "Copied" shows after the link is copied. */
const COPIED_MS = 1500;

/** Six hex digits: the generator's seed, and short enough to read as the puzzle's id. */
function randomSeed(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(3));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** A new random puzzle's id at `level`. The generator drops `-medium`. */
function newId(level: Level): string {
  return `${randomSeed()}-${level}`;
}

// The address bar holds the random puzzle's id: `?seed=<id>` exactly while Random is on,
// so the page's own URL is always a link to the puzzle on screen.
const listeners = new Set<() => void>();

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function urlSeed(): string | null {
  return seedParam(location.search);
}

/**
 * Puts `id` in the address bar, or takes the seed out for null, without a new entry.
 * Next's router picks the new URL up (and keeps its own history state) itself.
 */
function setUrlSeed(id: string | null) {
  history.replaceState(null, "", withSeed(location.href, id));
  listeners.forEach((l) => l());
}

/**
 * Tells the store when Next changes the URL itself, as the header's Game link does from a
 * ?seed=. Behind Suspense, as Next asks, since a static page has no search params.
 */
function UrlWatcher() {
  const params = useSearchParams();
  useEffect(() => {
    listeners.forEach((l) => l());
  }, [params]);
  return null;
}

/**
 * The game page's puzzle: today's daily, or a random one, both generated in the browser;
 * the random one has its solution behind a disclosure. Both boards stay mounted, so
 * switching keeps progress.
 */
export function GameModes() {
  // Null while prerendering, so the static page is the daily and hydrates as it.
  const seed = useSyncExternalStore(subscribe, urlSeed, () => null);
  const mode: Mode = seed ? "random" : "daily";
  const daily = useDailyPuzzle(useToday());
  const [random, setRandom] = useState<Puzzle | null>(null);
  // Mounted once shown, then kept. Mounted hidden, as when the page opens on a ?seed=,
  // its board would size its canvas to nothing.
  const [dailyShown, setDailyShown] = useState(false);
  const puzzle = mode === "daily" ? daily : random;

  // Generates the puzzle named in the address bar, then writes back its id as the
  // generator spells it (`bob-medium` is `bob`).
  useEffect(() => {
    if (!seed || seed === random?.id) return;
    let stale = false;
    void generatePuzzle(seed).then((p) => {
      if (stale) return;
      if (p.id !== random?.id) setRandom(p);
      if (p.id !== seed) setUrlSeed(p.id);
    });
    return () => {
      stale = true;
    };
  }, [seed, random]);

  const choose = (m: Mode) => {
    if (m === mode) return;
    if (mode === "daily") setDailyShown(true);
    // The first random puzzle is Medium, whose id is the bare seed.
    setUrlSeed(m === "random" ? (random?.id ?? randomSeed()) : null);
  };

  return (
    <>
      <Suspense>
        <UrlWatcher />
      </Suspense>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        {mode === "daily" ? "Daily" : "Puzzle"} {puzzle?.id ?? "…"}
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Fill the 4×4×4 cube with boxes. Every box holds exactly one clue and
        takes that clue&apos;s shape and volume.
      </p>
      <div className="mt-12 flex items-center justify-between gap-4">
        <div
          role="group"
          aria-label="Puzzle"
          className="inline-flex rounded-md border border-white/15 p-0.5 text-sm font-medium"
        >
          {MODES.map(([m, label]) => (
            <button
              key={m}
              type="button"
              aria-pressed={mode === m}
              onClick={() => choose(m)}
              className={`rounded px-3 py-1.5 ${mode === m ? "bg-white text-black" : "text-zinc-400 hover:text-white"}`}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-4">
          <Link
            href="/history"
            className="text-sm text-zinc-400 hover:text-white"
          >
            History
          </Link>
          <Tutorial />
        </div>
      </div>
      {daily && (mode === "daily" || dailyShown) && (
        <div className="mt-6" hidden={mode !== "daily"}>
          <Cube puzzle={daily} mode="play" day={daily.id} />
        </div>
      )}
      <RandomMode hidden={mode !== "random"} seed={seed} puzzle={random} />
      {puzzle && (
        <details className="mt-16">
          <summary className="cursor-pointer text-sm text-zinc-400 hover:text-white">
            Clues layer by layer (layer 1 is the bottom)
          </summary>
          <div className="mt-6">
            <Layers puzzle={puzzle} showSolution={false} />
          </div>
        </details>
      )}
    </>
  );
}

/**
 * Random mode: the level, a box to type a seed or id into, and a link to share, over the
 * random puzzle's board. `seed` is the id in the address bar; `puzzle` is null until the
 * first one is generated.
 */
function RandomMode({
  hidden,
  seed,
  puzzle,
}: {
  hidden: boolean;
  seed: string | null;
  puzzle: Puzzle | null;
}) {
  const level = puzzle?.level ?? "medium";
  const [draft, setDraft] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const id = setTimeout(() => setCopied(false), COPIED_MS);
    return () => clearTimeout(id);
  }, [copied]);

  const load = (id: string) => {
    setDraft(null);
    setUrlSeed(id);
  };
  const pickLevel = (l: Level) => {
    if (l !== level) load(newId(l));
  };
  // A blank box just shows the current id again.
  const loadDraft = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (draft?.trim()) load(draft);
    else setDraft(null);
  };
  const copyLink = () => {
    void navigator.clipboard
      .writeText(location.href)
      .then(() => setCopied(true));
  };

  return (
    <div hidden={hidden}>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <div
          role="group"
          aria-label="Level"
          className="inline-flex rounded-md border border-white/15 p-0.5 text-sm font-medium"
        >
          {LEVELS.map(([l, label]) => (
            <button
              key={l}
              type="button"
              aria-pressed={level === l}
              onClick={() => pickLevel(l)}
              className={`rounded px-3 py-1.5 ${level === l ? "bg-white text-black" : "text-zinc-400 hover:text-white"}`}
            >
              {label}
            </button>
          ))}
        </div>
        <form onSubmit={loadDraft} className="flex items-center gap-2">
          <label htmlFor="seed" className="text-sm text-zinc-400">
            Seed
          </label>
          {/* Seeds are case-sensitive, so no capitalising or correcting keyboards. */}
          <input
            id="seed"
            name="seed"
            value={draft ?? seed ?? ""}
            onChange={(e) => setDraft(e.target.value)}
            autoComplete="off"
            autoCapitalize="none"
            autoCorrect="off"
            spellCheck={false}
            enterKeyHint="go"
            className="w-36 rounded-md border border-white/15 bg-transparent px-2 py-2 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-white/40"
          />
        </form>
        <button
          type="button"
          className="rounded-md border border-white/15 px-4 py-2 text-sm font-medium whitespace-nowrap hover:bg-white/10"
          onClick={copyLink}
        >
          {copied ? "Copied" : "Copy link"}
        </button>
      </div>
      {puzzle && (
        <RandomBoard
          key={puzzle.id}
          puzzle={puzzle}
          onNew={() => load(newId(level))}
        />
      )}
    </div>
  );
}

/** The random puzzle's board, and its solution behind a disclosure that starts closed. */
function RandomBoard({ puzzle, onNew }: { puzzle: Puzzle; onNew: () => void }) {
  return (
    <div className="mt-6">
      <Cube
        puzzle={puzzle}
        mode="play"
        solvedNote="Every cell is in exactly one box."
        actions={
          <button
            type="button"
            className="rounded-md border border-white/15 px-4 py-2 text-sm font-medium whitespace-nowrap hover:bg-white/10"
            onClick={onNew}
          >
            New puzzle
          </button>
        }
      />
      <Solution puzzle={puzzle} />
    </div>
  );
}
