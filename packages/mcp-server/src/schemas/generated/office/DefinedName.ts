import { z } from "zod"

export const DefinedNameSchema = z.object({ "name": z.string().describe("e.g. \"WACC\", \"DCF_FAIR_VALUE\""), "range": z.string().describe("Range in A1 form, e.g. \"Sheet1!$B$5\" or \"Comps!$D$2:$D$10\".") }).describe("A workbook-level named range / defined name.")
