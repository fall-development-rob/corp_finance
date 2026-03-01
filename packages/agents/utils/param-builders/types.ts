// Shared types and utilities for param builders

import type { ExtractedMetrics } from '../financial-parser.js';

export type ParamBuilder = (m: ExtractedMetrics) => Record<string, unknown>;

/** Track which param values are real data vs estimates */
export interface ParamQuality {
  realFields: string[];
  estimatedFields: string[];
  missingCriticalFields: string[];
}

export function trackQuality(m: ExtractedMetrics, required: string[], optional: string[]): ParamQuality {
  const quality: ParamQuality = { realFields: [], estimatedFields: [], missingCriticalFields: [] };

  const mRec = m as unknown as Record<string, unknown>;

  for (const field of required) {
    const val = mRec[field];
    if (val !== undefined && val !== null) {
      quality.realFields.push(field);
    } else {
      quality.missingCriticalFields.push(field);
    }
  }

  for (const field of optional) {
    const val = mRec[field];
    if (val !== undefined && val !== null) {
      quality.realFields.push(field);
    } else {
      quality.estimatedFields.push(field);
    }
  }

  return quality;
}
