// Shared types for geopolitical-mcp-server

export interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

export type AlertSeverity = 'green' | 'orange' | 'red';

export type ConflictType =
  | 'battles'
  | 'protests'
  | 'riots'
  | 'explosions'
  | 'violence_against_civilians'
  | 'strategic_developments';

export type HazardType =
  | 'earthquake'
  | 'flood'
  | 'cyclone'
  | 'volcano'
  | 'drought'
  | 'wildfire';

export type AnomalySeverity = 'normal' | 'moderate' | 'severe' | 'extreme';

export type SentimentClassification =
  | 'extreme_fear'
  | 'fear'
  | 'neutral'
  | 'greed'
  | 'extreme_greed';

export type BarrierType =
  | 'tariff'
  | 'sps'
  | 'tbt'
  | 'anti_dumping'
  | 'countervailing'
  | 'safeguard'
  | 'quota';

export type AwardType =
  | 'contract'
  | 'grant'
  | 'loan'
  | 'direct_payment'
  | 'other';

export type DisplacementType =
  | 'refugee'
  | 'idp'
  | 'asylum_seeker'
  | 'stateless'
  | 'returned';

export { wrapResponse } from '@robotixai/mcp-utils';
