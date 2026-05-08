import { z } from "zod"

export const ChartSeriesSchema = z.object({ "categories_range": z.string().describe("A1 absolute range for the category (X-axis) data."), "name": z.string().describe("Series display name."), "values_range": z.string().describe("A1 absolute range for the values (Y-axis) data.") }).describe("One data series within a chart, specified via A1-style absolute ranges.\nExample: `categories_range = \"Sheet1!$A$2:$A$10\"`.")
