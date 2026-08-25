import { CREATORS, PATTERNS, TAGS } from "../fixtures/catalog.ts";
import type { Creator, Pattern } from "./types.ts";
import type { CreatorId, PatternId } from "./ids.ts";

// Build-time fixture, read-only for the whole app (state ownership table,
// section 5 of the round-1 IA review). No provider needed: nothing writes here.

export const catalogTags: readonly string[] = TAGS;

export function allPatterns(): Pattern[] {
  return PATTERNS;
}

export function findPattern(id: string): Pattern | undefined {
  return PATTERNS.find((pattern) => pattern.id === (id as PatternId));
}

export function findCreator(id: string): Creator | undefined {
  return CREATORS.find((creator) => creator.id === (id as CreatorId));
}

export function patternsByCreator(id: string): Pattern[] {
  return PATTERNS.filter((pattern) => pattern.creatorId === (id as CreatorId));
}

export interface PatternFilter {
  tag?: string | null;
  favoritesOnly?: boolean;
  favorites?: readonly PatternId[];
}

export function filterPatterns(patterns: Pattern[], filter: PatternFilter): Pattern[] {
  const favorites = filter.favorites ?? [];
  return patterns.filter((pattern) => {
    if (filter.tag && !pattern.tags.includes(filter.tag)) return false;
    if (filter.favoritesOnly && !favorites.includes(pattern.id)) return false;
    return true;
  });
}
