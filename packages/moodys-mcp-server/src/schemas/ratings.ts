import { IssuerIdentifierSchema, DateRangeSchema } from './common.js';

export const CreditRatingSchema = IssuerIdentifierSchema;

export const RatingHistorySchema = IssuerIdentifierSchema.merge(DateRangeSchema);

export const IssuerProfileSchema = IssuerIdentifierSchema;
