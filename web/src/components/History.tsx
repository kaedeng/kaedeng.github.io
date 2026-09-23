"use client";

import { useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Cube, formatTime } from "@/components/Cube";
import { Solution } from "@/components/Solution";
import {
  addMonths,
  calendar,
  FIRST_DAY,
  monthName,
  monthOf,
  pastDay,
} from "@/lib/daily";
import { useSolves, type Solves } from "@/lib/solves";
import { useDailyPuzzle, useToday } from "@/lib/useDaily";

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/**
 * A month of dailies with this browser's solve times, and the daily picked in `?day=`
 * under it to play. Behind Suspense, as Next asks, since a static page has no search
 * params.
 */
export function History() {
  const today = useToday();
  const asked = useSearchParams().get("day");
  const day = today && pastDay(asked, today);
  const puzzle = useDailyPuzzle(day);
  const solves = useSolves();
  // Null until the player turns the page: the picked day's month, else today's.
  const [month, setMonth] = useState<string | null>(null);
  if (!today) return null;

  return (
    <>
      <Calendar
        month={month ?? monthOf(day ?? today)}
        today={today}
        picked={day}
        solves={solves ?? {}}
        onMonth={setMonth}
      />
      {day && (
        <section key={day} className="mt-16">
          <h2 className="text-2xl font-semibold tracking-tight">Daily {day}</h2>
          {puzzle && (
            <div className="mt-6">
              <Cube puzzle={puzzle} mode="play" day={day} />
              <Solution puzzle={puzzle} />
            </div>
          )}
        </section>
      )}
    </>
  );
}

function Calendar({
  month,
  today,
  picked,
  solves,
  onMonth,
}: {
  month: string;
  today: string;
  picked: string | null;
  solves: Solves;
  onMonth: (month: string) => void;
}) {
  return (
    <div className="mt-12 max-w-md">
      <div className="flex items-center justify-between">
        <MonthButton
          label="Previous month"
          glyph="‹"
          to={addMonths(month, -1)}
          disabled={month <= monthOf(FIRST_DAY)}
          onMonth={onMonth}
        />
        <h2 className="text-sm font-medium">{monthName(month)}</h2>
        <MonthButton
          label="Next month"
          glyph="›"
          to={addMonths(month, 1)}
          disabled={month >= monthOf(today)}
          onMonth={onMonth}
        />
      </div>
      <div className="mt-4 grid grid-cols-7 gap-1 text-center">
        {WEEKDAYS.map((name) => (
          <abbr
            key={name}
            title={name}
            className="pb-1 text-xs text-zinc-500 no-underline"
          >
            {name[0]}
          </abbr>
        ))}
        {calendar(month).map((d, i) =>
          d === null ? (
            <span key={`blank-${i}`} />
          ) : (
            <Day
              key={d}
              day={d}
              today={today}
              picked={d === picked}
              time={solves[d]}
            />
          ),
        )}
      </div>
    </div>
  );
}

function MonthButton({
  label,
  glyph,
  to,
  disabled,
  onMonth,
}: {
  label: string;
  glyph: string;
  to: string;
  disabled: boolean;
  onMonth: (month: string) => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={() => onMonth(to)}
      className="rounded-md px-3 py-1 text-zinc-400 hover:text-white disabled:opacity-30 disabled:hover:text-zinc-400"
    >
      {glyph}
    </button>
  );
}

/** One day: a link to its daily once it is out, with the time it was solved in. */
function Day({
  day,
  today,
  picked,
  time,
}: {
  day: string;
  today: string;
  picked: boolean;
  time: number | undefined;
}) {
  const date = Number(day.slice(8));
  const cell =
    "flex aspect-square flex-col items-center justify-center rounded-md text-sm tabular-nums";
  if (day < FIRST_DAY || day > today) {
    return <span className={`${cell} text-zinc-700`}>{date}</span>;
  }
  const look = picked
    ? "bg-white text-black"
    : "text-zinc-300 hover:bg-white/10 hover:text-white";
  return (
    <Link
      href={`/history?day=${day}`}
      aria-current={picked ? "date" : undefined}
      aria-label={`${day}${time === undefined ? "" : `, solved in ${formatTime(time)}`}`}
      className={`${cell} ${look} ${day === today ? "ring-1 ring-white/40" : ""}`}
    >
      {date}
      {time !== undefined && (
        <span
          className={`font-mono text-[10px] leading-none ${picked ? "text-black/60" : "text-[#93ce70]"}`}
        >
          {formatTime(time)}
        </span>
      )}
    </Link>
  );
}
