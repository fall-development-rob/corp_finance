import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { eiaFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const PetroleumSeriesEnum = z.enum([
  'crude_production',
  'crude_inventory',
  'refinery_throughput',
  'imports',
  'exports',
  'spr',
]);

const PetroleumSchema = z.object({
  series: PetroleumSeriesEnum.describe('Petroleum data series to query'),
  frequency: z.enum(['monthly', 'weekly']).default('weekly').describe('Data frequency'),
  limit: z.number().int().min(1).max(500).default(52).describe('Number of records to return'),
});

const ElectricityFuelEnum = z.enum([
  'coal',
  'natural_gas',
  'nuclear',
  'solar',
  'wind',
  'hydro',
  'total',
]);

const ElectricitySchema = z.object({
  fuel_type: ElectricityFuelEnum.optional().describe('Filter by fuel/energy source'),
  frequency: z.enum(['monthly', 'annual']).default('monthly').describe('Data frequency'),
  limit: z.number().int().min(1).max(500).default(24).describe('Number of records to return'),
});

const CapacitySchema = z.object({
  energy_source: z.string().optional().describe('Filter by energy source (e.g., solar, wind, nuclear)'),
  limit: z.number().int().min(1).max(500).default(20).describe('Number of records to return'),
});

// ---------- EIA v2 series-to-product mapping ----------

const PETROLEUM_PRODUCT_MAP: Record<z.infer<typeof PetroleumSeriesEnum>, string> = {
  crude_production: 'EPC0',
  crude_inventory: 'SAE',
  refinery_throughput: 'YUP',
  imports: 'EPC0',
  exports: 'EPC0',
  spr: 'WCSSTUS1',
};

const PETROLEUM_PROCESS_MAP: Record<z.infer<typeof PetroleumSeriesEnum>, string> = {
  crude_production: 'FPF',
  crude_inventory: 'SAE',
  refinery_throughput: 'YUP',
  imports: 'IM0',
  exports: 'EX0',
  spr: 'SAE',
};

const FUEL_TYPE_MAP: Record<string, string> = {
  coal: 'COW',
  natural_gas: 'NG',
  nuclear: 'NUC',
  solar: 'SUN',
  wind: 'WND',
  hydro: 'HYC',
  total: 'ALL',
};

// ---------- Response types ----------

interface EiaV2Response {
  response: {
    data: Record<string, unknown>[];
    total?: number;
    dateFormat?: string;
  };
}

// ---------- Registration ----------

export function registerEiaTools(server: McpServer) {
  // --- eia_petroleum ---
  server.tool(
    'eia_petroleum',
    'US petroleum supply & demand data from EIA. Query crude production, inventory, refinery throughput, imports/exports, or SPR levels. Weekly or monthly frequency.',
    PetroleumSchema.shape,
    async (params) => {
      const { series, frequency, limit } = PetroleumSchema.parse(params);

      const route = frequency === 'weekly'
        ? 'petroleum/sum/sndw/data/'
        : 'petroleum/sum/snd/data/';

      const queryParams: Record<string, string | number> = {
        'frequency': frequency,
        'data[0]': 'value',
        'sort[0][column]': 'period',
        'sort[0][direction]': 'desc',
        'offset': 0,
        'length': limit,
      };

      // Filter by product/process to narrow to the requested series
      const product = PETROLEUM_PRODUCT_MAP[series];
      const process = PETROLEUM_PROCESS_MAP[series];
      if (product) queryParams['facets[product][]'] = product;
      if (process) queryParams['facets[process][]'] = process;

      const raw = await eiaFetch<EiaV2Response>(route, queryParams, {
        cacheTtl: frequency === 'weekly' ? CacheTTL.SHORT : CacheTTL.MEDIUM,
      });

      const records = (raw.response?.data ?? []).map(d => ({
        period: d.period,
        value: d.value,
        unit: d['units'] ?? d['unit'] ?? null,
        series_description: d['series-description'] ?? d['seriesDescription'] ?? series,
      }));

      return wrapResponse({
        source: 'EIA',
        series,
        frequency,
        count: records.length,
        data: records,
      });
    },
  );

  // --- eia_electricity ---
  server.tool(
    'eia_electricity',
    'US electricity generation data from EIA. Filter by fuel type (coal, natural gas, nuclear, solar, wind, hydro). Monthly or annual frequency.',
    ElectricitySchema.shape,
    async (params) => {
      const { fuel_type, frequency, limit } = ElectricitySchema.parse(params);

      const route = 'electricity/electric-power-operational-data/data/';

      const queryParams: Record<string, string | number> = {
        'frequency': frequency,
        'data[0]': 'generation',
        'sort[0][column]': 'period',
        'sort[0][direction]': 'desc',
        'offset': 0,
        'length': limit,
      };

      if (fuel_type) {
        const fuelCode = FUEL_TYPE_MAP[fuel_type];
        if (fuelCode) queryParams['facets[fueltypeid][]'] = fuelCode;
      }

      // Sector: electric utility + independent power producers
      queryParams['facets[sectorid][]'] = '99'; // All sectors

      const raw = await eiaFetch<EiaV2Response>(route, queryParams, {
        cacheTtl: CacheTTL.MEDIUM,
      });

      const records = (raw.response?.data ?? []).map(d => ({
        period: d.period,
        generation: d['generation'] ?? d['value'] ?? null,
        capacity_factor: d['capacity-factor'] ?? d['capacityFactor'] ?? null,
        unit: d['units'] ?? d['unit'] ?? 'thousand megawatthours',
        fuel_type: d['fuelTypeDescription'] ?? d['fueltypeid'] ?? fuel_type ?? null,
      }));

      return wrapResponse({
        source: 'EIA',
        fuel_type: fuel_type ?? 'all',
        frequency,
        count: records.length,
        data: records,
      });
    },
  );

  // --- eia_capacity ---
  server.tool(
    'eia_capacity',
    'US power plant operating generator capacity from EIA. Filter by energy source. Returns plant-level nameplate capacity data.',
    CapacitySchema.shape,
    async (params) => {
      const { energy_source, limit } = CapacitySchema.parse(params);

      const route = 'electricity/operating-generator-capacity/data/';

      const queryParams: Record<string, string | number> = {
        'data[0]': 'nameplate-capacity-mw',
        'sort[0][column]': 'nameplate-capacity-mw',
        'sort[0][direction]': 'desc',
        'offset': 0,
        'length': limit,
      };

      if (energy_source) {
        queryParams['facets[energy_source_code][]'] = energy_source.toUpperCase();
      }

      const raw = await eiaFetch<EiaV2Response>(route, queryParams, {
        cacheTtl: CacheTTL.STATIC,
      });

      const records = (raw.response?.data ?? []).map(d => ({
        plant_name: d['plantName'] ?? d['plant_name'] ?? d['entityName'] ?? null,
        state: d['stateid'] ?? d['state'] ?? null,
        energy_source: d['energy_source_code'] ?? d['energySourceDescription'] ?? energy_source ?? null,
        nameplate_capacity_mw: d['nameplate-capacity-mw'] ?? d['nameplate_capacity_mw'] ?? null,
        status: d['status'] ?? d['statusDescription'] ?? null,
      }));

      return wrapResponse({
        source: 'EIA',
        energy_source: energy_source ?? 'all',
        count: records.length,
        data: records,
      });
    },
  );
}
