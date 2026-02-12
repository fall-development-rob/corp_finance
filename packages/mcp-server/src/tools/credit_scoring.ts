import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateScorecard,
  calculateMerton,
  calculateIntensityModel,
  calculateCalibration,
  calculateScoringValidation,
} from "@fall-development-rob/corp-finance-bindings";
import {
  CreditScorecardSchema,
  MertonPdSchema,
  IntensityModelSchema,
  PdCalibrationSchema,
  ScoringValidationSchema,
} from "../schemas/credit_scoring.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCreditScoringTools(server: McpServer) {
  server.tool(
    "credit_scorecard",
    "Logistic regression scorecard: WoE binning, IV calculation, scorecard points, Gini coefficient, KS statistic",
    CreditScorecardSchema.shape,
    async (params) => {
      const validated = CreditScorecardSchema.parse(coerceNumbers(params));
      const result = calculateScorecard(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "merton_pd",
    "Merton structural model: asset value/volatility estimation, distance to default, PD, KMV EDF",
    MertonPdSchema.shape,
    async (params) => {
      const validated = MertonPdSchema.parse(coerceNumbers(params));
      const result = calculateMerton(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "intensity_model",
    "Reduced-form intensity model: hazard rates from CDS spreads, survival probability, term structure",
    IntensityModelSchema.shape,
    async (params) => {
      const validated = IntensityModelSchema.parse(coerceNumbers(params));
      const result = calculateIntensityModel(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "pd_calibration",
    "PIT/TTC PD calibration: Vasicek single-factor model, Basel IRB correlation, central tendency",
    PdCalibrationSchema.shape,
    async (params) => {
      const validated = PdCalibrationSchema.parse(coerceNumbers(params));
      const result = calculateCalibration(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "scoring_validation",
    "Credit model validation: AUC-ROC, accuracy ratio, Gini, Brier score, Hosmer-Lemeshow test",
    ScoringValidationSchema.shape,
    async (params) => {
      const validated = ScoringValidationSchema.parse(coerceNumbers(params));
      const result = calculateScoringValidation(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
