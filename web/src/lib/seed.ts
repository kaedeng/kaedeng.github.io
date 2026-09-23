/**
 * The puzzle id in a `?seed=` query, or null when there is none or it is blank. It comes
 * back as written: the generator trims it and reads its level.
 */
export function seedParam(search: string): string | null {
  const id = new URLSearchParams(search).get("seed");
  return id?.trim() ? id : null;
}

/** `href` with its `?seed=` set to `id`, or taken out for null. */
export function withSeed(href: string, id: string | null): string {
  const url = new URL(href);
  if (id === null) url.searchParams.delete("seed");
  else url.searchParams.set("seed", id);
  return url.href;
}
