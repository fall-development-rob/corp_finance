import { z } from 'zod';

// ---------------------------------------------------------------------------
// SSCMFI Bond Math — Zod schemas matching SscmfiBondInput in sscmfi.rs
// ---------------------------------------------------------------------------

const CallRedemptionSchema = z.object({
  date: z.string().describe('Call date in MM/DD/YYYY format'),
  price: z.coerce.number().positive().describe('Call price per 100 par'),
});

const StepScheduleSchema = z.object({
  date: z.string().describe('Step date in MM/DD/YYYY format'),
  coupon_rate: z.coerce.number().describe('New coupon rate as percentage (e.g. 5.0 = 5%)'),
});

// Full SSCMFI bond input schema
export const SscmfiBondSchema = z.object({
  security_type: z.enum(['Treasury', 'Agency', 'Corporate', 'Municipal', 'CD'])
    .describe('Security type — determines default day count, frequency, and EOM conventions'),
  payment_type: z.enum(['Periodic', 'Discount', 'IAM', 'Stepped', 'Multistep', 'PIK', 'PartPIK'])
    .optional().default('Periodic')
    .describe('Payment type (default: Periodic)'),
  maturity_date: z.string()
    .describe('Maturity date in MM/DD/YYYY format'),
  coupon_rate: z.coerce.number()
    .describe('Annual coupon rate as percentage (e.g. 5.0 = 5%)'),
  given_type: z.enum(['Price', 'Yield'])
    .describe('Whether the given_value is a price or yield'),
  given_value: z.coerce.number()
    .describe('The known value — price per 100 par or yield as percentage'),
  settlement_date: z.string().optional()
    .describe('Settlement date in MM/DD/YYYY format (default: today)'),
  redemption_value: z.coerce.number().positive().optional()
    .describe('Redemption value per 100 par (default: 100)'),
  day_count: z.enum(['SSCM30_360', 'ActualActual', 'Actual360', 'Actual365']).optional()
    .describe('Day count convention (defaults by security type)'),
  eom_rule: z.enum(['Adjust', 'NoAdjust']).optional()
    .describe('End-of-month rule (default: Adjust)'),
  frequency: z.enum(['Annual', 'Semiannual', 'Quarterly', 'Monthly']).optional()
    .describe('Coupon frequency (defaults by security type)'),
  call_schedule: z.array(CallRedemptionSchema).optional()
    .describe('Call schedule for callable bonds — array of {date, price}'),
  step_schedule: z.array(StepScheduleSchema).optional()
    .describe('Step schedule for Stepped/Multistep bonds — array of {date, coupon_rate}'),
  pik_rate: z.coerce.number().optional()
    .describe('PIK rate as percentage for PIK/PartPIK bonds'),
  cash_rate: z.coerce.number().optional()
    .describe('Cash coupon rate as percentage for PartPIK bonds'),
  calc_analytics: z.coerce.boolean().optional().default(true)
    .describe('Calculate duration, convexity, PV01, YV32 (default: true)'),
  calc_cashflows: z.coerce.boolean().optional().default(false)
    .describe('Generate full cashflow schedule (default: false)'),
});

// Batch schema for multiple bonds
export const SscmfiBatchSchema = z.object({
  bonds: z.array(SscmfiBondSchema).min(1).max(100)
    .describe('Array of SSCMFI bond inputs (max 100 per batch)'),
});

// Quick price-to-yield shortcut schema
export const SscmfiPriceToYieldSchema = z.object({
  security_type: z.enum(['Treasury', 'Agency', 'Corporate', 'Municipal', 'CD'])
    .describe('Security type'),
  maturity_date: z.string()
    .describe('Maturity date in MM/DD/YYYY format'),
  coupon_rate: z.coerce.number()
    .describe('Annual coupon rate as percentage'),
  price: z.coerce.number()
    .describe('Clean price per 100 par'),
  settlement_date: z.string().optional()
    .describe('Settlement date in MM/DD/YYYY format'),
  call_schedule: z.array(CallRedemptionSchema).optional()
    .describe('Call schedule for callable bonds'),
});

// Quick yield-to-price shortcut schema
export const SscmfiYieldToPriceSchema = z.object({
  security_type: z.enum(['Treasury', 'Agency', 'Corporate', 'Municipal', 'CD'])
    .describe('Security type'),
  maturity_date: z.string()
    .describe('Maturity date in MM/DD/YYYY format'),
  coupon_rate: z.coerce.number()
    .describe('Annual coupon rate as percentage'),
  yield_value: z.coerce.number()
    .describe('Yield as percentage'),
  settlement_date: z.string().optional()
    .describe('Settlement date in MM/DD/YYYY format'),
});
