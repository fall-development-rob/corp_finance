import { z } from "zod";

export const KycRiskSchema = z.object({
  customer_name: z.string().describe("Customer name"),
  customer_type: z.enum([
    "Individual",
    "Corporate",
    "Trust",
    "Foundation",
    "Partnership",
    "PEP",
    "ComplexStructure",
  ]).describe("Customer type classification"),
  jurisdiction_of_incorporation: z.string().describe("Jurisdiction of incorporation or nationality"),
  jurisdiction_of_operations: z.array(z.string()).describe("Jurisdictions where customer operates"),
  is_pep: z.boolean().describe("Whether the customer is a Politically Exposed Person"),
  pep_category: z.enum([
    "DomesticPEP",
    "ForeignPEP",
    "InternationalOrgPEP",
    "FamilyMember",
    "CloseAssociate",
  ]).optional().describe("PEP category if applicable"),
  years_since_pep_role: z.coerce.number().int().min(0).optional().describe("Years since the person left PEP role"),
  source_of_wealth: z.enum([
    "Employment",
    "Business",
    "Inheritance",
    "Investment",
    "Unclear",
    "HighRiskIndustry",
  ]).describe("Primary source of wealth"),
  source_of_funds: z.string().describe("Description of the source of funds"),
  product_type: z.enum([
    "RetailBanking",
    "PrivateBanking",
    "CorrespondentBanking",
    "TradeFinance",
    "FundInvestment",
    "CustodyServices",
  ]).describe("Type of financial product or service"),
  channel: z.enum([
    "FaceToFace",
    "Online",
    "IntroducedBusiness",
    "ThirdParty",
  ]).describe("Channel through which the relationship was established"),
  annual_transaction_volume: z.coerce.number().min(0).describe("Expected annual transaction volume"),
  average_transaction_size: z.coerce.number().min(0).describe("Expected average transaction size"),
  cross_border_transaction_pct: z.coerce.number().min(0).max(1).describe("Cross-border transaction percentage (decimal)"),
  cash_transaction_pct: z.coerce.number().min(0).max(1).describe("Cash transaction percentage (decimal)"),
  ownership_layers: z.coerce.number().int().min(0).describe("Number of ownership layers in the corporate structure"),
  has_nominee_directors: z.boolean().describe("Whether nominee directors are used"),
  has_bearer_shares: z.boolean().describe("Whether bearer shares exist in the structure"),
  adverse_media_hits: z.coerce.number().int().min(0).describe("Number of adverse media hits found"),
  industry: z.string().describe("Customer industry or business sector"),
  expected_account_activity: z.string().describe("Description of expected account activity"),
});

const ScreeningEntitySchema = z.object({
  name: z.string().describe("Entity or individual name to screen"),
  aliases: z.array(z.string()).describe("Known aliases or alternative names"),
  date_of_birth: z.string().optional().describe("Date of birth (ISO format, for individuals)"),
  nationality: z.string().optional().describe("Nationality or country of incorporation"),
  jurisdiction: z.string().describe("Jurisdiction of the entity"),
  entity_type: z.enum([
    "Individual",
    "Corporate",
    "Trust",
    "Government",
    "Vessel",
    "Aircraft",
  ]).describe("Type of entity being screened"),
});

const TransactionDetailsSchema = z.object({
  amount: z.coerce.number().min(0).describe("Transaction amount"),
  currency: z.string().describe("Transaction currency code"),
  counterparty_jurisdiction: z.string().describe("Counterparty jurisdiction"),
  purpose: z.string().describe("Stated purpose of the transaction"),
});

export const SanctionsScreeningSchema = z.object({
  screening_type: z.enum([
    "Onboarding",
    "Transaction",
    "PeriodicReview",
    "BatchRescreen",
  ]).describe("Type of screening being performed"),
  entities_to_screen: z.array(ScreeningEntitySchema).describe("Entities or individuals to screen"),
  lists_to_check: z.array(z.enum([
    "OFAC_SDN",
    "EU_Consolidated",
    "HMT_UK",
    "UN_UNSC",
    "FATF_GreyList",
    "FATF_BlackList",
  ])).describe("Sanctions and watchlists to check against"),
  transaction_details: TransactionDetailsSchema.optional().describe("Transaction details for transaction screening"),
  screening_threshold: z.coerce.number().min(0).max(1).describe("Fuzzy matching threshold (decimal, e.g. 0.85)"),
  include_pep_screening: z.boolean().describe("Whether to include PEP screening"),
  include_adverse_media: z.boolean().describe("Whether to include adverse media screening"),
});
