import type { Metadata } from "next";
import { Suspense } from "react";
import { History } from "@/components/History";

export const metadata: Metadata = {
  title: "3D Patches · History",
};

export default function HistoryPage() {
  return (
    <>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        History
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Every daily puzzle so far, one a day from midnight Pacific. Pick a day
        to play it.
      </p>
      <Suspense>
        <History />
      </Suspense>
    </>
  );
}
