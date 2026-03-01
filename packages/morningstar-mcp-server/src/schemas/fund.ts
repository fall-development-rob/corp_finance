import { z } from 'zod';
import { FundIdentifierSchema, DateRangeSchema } from './common.js';

export const FundRatingSchema = FundIdentifierSchema;

export const FundHoldingsSchema = FundIdentifierSchema;

export const FundPerformanceSchema = FundIdentifierSchema;

export const HistoricalNavSchema = FundIdentifierSchema.merge(DateRangeSchema);

export const ExpenseSchema = FundIdentifierSchema;
