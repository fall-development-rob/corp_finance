import { z } from "zod";

export const LeaseClassificationSchema = z.object({
  lease_description: z.string().describe("Description of the lease"),
  standard: z.enum(["Asc842", "Ifrs16"]).describe("Accounting standard"),
  lease_term_months: z.coerce.number().int().positive().describe("Total lease term in months"),
  monthly_payment: z.coerce.number().positive().describe("Base monthly payment"),
  annual_escalation: z.coerce.number().min(0).optional().describe("Annual payment escalation rate"),
  incremental_borrowing_rate: z.coerce.number().positive().describe("Lessee IBR (annual)"),
  implicit_rate: z.coerce.number().positive().optional().describe("Rate implicit in the lease"),
  fair_value_of_asset: z.coerce.number().positive().describe("Fair market value of underlying asset"),
  useful_life_months: z.coerce.number().int().positive().describe("Economic useful life in months"),
  residual_value_guaranteed: z.coerce.number().min(0).optional().describe("Guaranteed residual value"),
  residual_value_unguaranteed: z.coerce.number().min(0).optional().describe("Unguaranteed residual value"),
  purchase_option_price: z.coerce.number().positive().optional().describe("Purchase option price"),
  purchase_option_reasonably_certain: z.boolean().optional().describe("Whether purchase option exercise is reasonably certain"),
  termination_penalty: z.coerce.number().min(0).optional().describe("Early termination penalty"),
  initial_direct_costs: z.coerce.number().min(0).optional().describe("Initial direct costs"),
  lease_incentives_received: z.coerce.number().min(0).optional().describe("Lease incentives received"),
  prepaid_lease_payments: z.coerce.number().min(0).optional().describe("Prepaid lease payments"),
  transfer_of_ownership: z.boolean().describe("Does ownership transfer at end?"),
  specialized_asset: z.boolean().describe("Is asset specialized with no alternative use?"),
});

export const SaleLeasebackSchema = z.object({
  description: z.string().describe("Transaction description"),
  standard: z.enum(["Asc842", "Ifrs16"]).describe("Accounting standard"),
  asset_carrying_value: z.coerce.number().min(0).describe("Book value of asset"),
  sale_price: z.coerce.number().positive().describe("Price buyer-lessor pays"),
  fair_value: z.coerce.number().positive().describe("Fair market value"),
  lease_term_months: z.coerce.number().int().positive().describe("Leaseback term in months"),
  monthly_lease_payment: z.coerce.number().positive().describe("Monthly leaseback payment"),
  annual_escalation: z.coerce.number().min(0).optional().describe("Annual payment escalation"),
  incremental_borrowing_rate: z.coerce.number().positive().describe("Lessee IBR (annual)"),
  useful_life_remaining_months: z.coerce.number().int().positive().describe("Remaining useful life in months"),
  qualifies_as_sale: z.boolean().describe("Whether transfer qualifies as sale under ASC 606"),
});
