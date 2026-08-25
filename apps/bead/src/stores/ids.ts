// R1 in the round-1 IA review: if the prefixes drift, `/pattern/:id` and
// `/assemble/:id` deep links cross-contaminate. The prefixes live in the types
// so a wrong id cannot be constructed silently, and in the tests so a later
// work package cannot quietly drop them.

export const ID_PREFIX = {
  pattern: "gal-",
  project: "proj-",
  creator: "cr-",
} as const;

declare const brand: unique symbol;

type Branded<T extends string> = string & { readonly [brand]: T };

export type PatternId = Branded<"PatternId">;
export type ProjectId = Branded<"ProjectId">;
export type CreatorId = Branded<"CreatorId">;

export function isPatternId(value: string): value is PatternId {
  return value.startsWith(ID_PREFIX.pattern) && value.length > ID_PREFIX.pattern.length;
}

export function isProjectId(value: string): value is ProjectId {
  return value.startsWith(ID_PREFIX.project) && value.length > ID_PREFIX.project.length;
}

export function isCreatorId(value: string): value is CreatorId {
  return value.startsWith(ID_PREFIX.creator) && value.length > ID_PREFIX.creator.length;
}

export function asPatternId(value: string): PatternId {
  if (!isPatternId(value)) throw new Error(`不是图纸 id（应以 ${ID_PREFIX.pattern} 开头）：${value}`);
  return value;
}

export function asProjectId(value: string): ProjectId {
  if (!isProjectId(value)) throw new Error(`不是项目 id（应以 ${ID_PREFIX.project} 开头）：${value}`);
  return value;
}

export function asCreatorId(value: string): CreatorId {
  if (!isCreatorId(value)) throw new Error(`不是创作者 id（应以 ${ID_PREFIX.creator} 开头）：${value}`);
  return value;
}

let mintCounter = 0;

/** Monotonic within a session; the timestamp keeps it unique across sessions. */
export function mintProjectId(now: number = Date.now()): ProjectId {
  mintCounter += 1;
  return `${ID_PREFIX.project}${now.toString(36)}-${mintCounter.toString(36)}` as ProjectId;
}
