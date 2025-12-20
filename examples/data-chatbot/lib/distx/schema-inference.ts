/**
 * Schema Inference for DistX
 * 
 * Automatically infers a Similarity Schema from tabular data.
 * Analyzes column types, cardinality, and distributions to create
 * an optimal Similarity Contract.
 */

import type {
  FieldConfig,
  FieldType,
  DistanceType,
  SimilaritySchema,
  PointPayload,
  InferredSchema,
} from './types';

interface ColumnStats {
  name: string;
  values: (string | number | boolean | null)[];
  uniqueCount: number;
  nullCount: number;
  numericCount: number;
  booleanCount: number;
  textLength: { min: number; max: number; avg: number };
}

/**
 * Analyze a column to gather statistics
 */
function analyzeColumn(
  name: string,
  values: (string | number | boolean | null)[]
): ColumnStats {
  const unique = new Set<string>();
  let nullCount = 0;
  let numericCount = 0;
  let booleanCount = 0;
  let totalLength = 0;
  let minLength = Infinity;
  let maxLength = 0;
  let textCount = 0;

  for (const value of values) {
    if (value === null || value === undefined || value === '') {
      nullCount++;
      continue;
    }

    unique.add(String(value));

    if (typeof value === 'boolean') {
      booleanCount++;
    } else if (typeof value === 'number' || !isNaN(Number(value))) {
      numericCount++;
    }

    const strValue = String(value);
    if (strValue.length > 0) {
      textCount++;
      totalLength += strValue.length;
      minLength = Math.min(minLength, strValue.length);
      maxLength = Math.max(maxLength, strValue.length);
    }
  }

  return {
    name,
    values,
    uniqueCount: unique.size,
    nullCount,
    numericCount,
    booleanCount,
    textLength: {
      min: minLength === Infinity ? 0 : minLength,
      max: maxLength,
      avg: textCount > 0 ? totalLength / textCount : 0,
    },
  };
}

/**
 * Infer field type from column statistics
 */
function inferFieldType(stats: ColumnStats): {
  type: FieldType;
  distance: DistanceType;
  reason: string;
} {
  const totalValues = stats.values.length;
  const nonNullCount = totalValues - stats.nullCount;

  if (nonNullCount === 0) {
    return {
      type: 'text',
      distance: 'semantic',
      reason: 'All null values, defaulting to text',
    };
  }

  // Check for boolean
  if (stats.booleanCount > nonNullCount * 0.8) {
    return {
      type: 'boolean',
      distance: 'exact',
      reason: `${Math.round((stats.booleanCount / nonNullCount) * 100)}% boolean values`,
    };
  }

  // Check for pure numeric
  if (stats.numericCount > nonNullCount * 0.9) {
    // Determine if absolute or relative distance is better
    // Use relative for prices, quantities, scores (high variance)
    // Use absolute for things like ratings (1-5, low variance)
    const distance = stats.uniqueCount > 10 ? 'relative' : 'absolute';
    return {
      type: 'number',
      distance,
      reason: `${Math.round((stats.numericCount / nonNullCount) * 100)}% numeric values, ${stats.uniqueCount} unique`,
    };
  }

  // Check for categorical (low cardinality)
  const cardinalityRatio = stats.uniqueCount / nonNullCount;
  if (cardinalityRatio < 0.1 || stats.uniqueCount <= 50) {
    return {
      type: 'categorical',
      distance: 'exact',
      reason: `Low cardinality: ${stats.uniqueCount} unique values (${Math.round(cardinalityRatio * 100)}% unique)`,
    };
  }

  // Default to text for high cardinality strings
  return {
    type: 'text',
    distance: 'semantic',
    reason: `High cardinality text: ${stats.uniqueCount} unique values, avg length ${Math.round(stats.textLength.avg)}`,
  };
}

/**
 * Calculate field weight based on importance heuristics
 */
function calculateWeight(
  fieldName: string,
  stats: ColumnStats,
  fieldType: FieldType,
  allStats: ColumnStats[]
): number {
  let weight = 1.0;

  // Boost name-like fields
  const nameLikePatterns = ['name', 'title', 'description', 'label'];
  if (nameLikePatterns.some((p) => fieldName.toLowerCase().includes(p))) {
    weight *= 1.5;
  }

  // Boost price/cost fields
  const pricePatterns = ['price', 'cost', 'amount', 'value'];
  if (pricePatterns.some((p) => fieldName.toLowerCase().includes(p))) {
    weight *= 1.3;
  }

  // Reduce weight for ID-like fields
  const idPatterns = ['id', 'uuid', 'guid', 'key'];
  if (
    idPatterns.some(
      (p) => fieldName.toLowerCase() === p || fieldName.toLowerCase().endsWith('_id')
    )
  ) {
    weight *= 0.1;
  }

  // Reduce weight for mostly null fields
  const nullRatio = stats.nullCount / stats.values.length;
  if (nullRatio > 0.5) {
    weight *= 0.5;
  }

  // Reduce weight for boolean (less discriminative)
  if (fieldType === 'boolean') {
    weight *= 0.7;
  }

  // Boost text fields with moderate length (likely names/titles)
  if (fieldType === 'text' && stats.textLength.avg > 5 && stats.textLength.avg < 100) {
    weight *= 1.2;
  }

  return weight;
}

/**
 * Normalize weights to sum to 1.0
 */
function normalizeWeights(
  fields: Record<string, FieldConfig>
): Record<string, FieldConfig> {
  const total = Object.values(fields).reduce((sum, f) => sum + f.weight, 0);
  if (total === 0) return fields;

  const normalized: Record<string, FieldConfig> = {};
  for (const [name, config] of Object.entries(fields)) {
    normalized[name] = {
      ...config,
      weight: Math.round((config.weight / total) * 100) / 100, // Round to 2 decimals
    };
  }
  return normalized;
}

/**
 * Infer a Similarity Schema from tabular data
 * 
 * @param data - Array of row objects
 * @param options - Configuration options
 * @returns Inferred schema with explanation
 */
export function inferSchema(
  data: PointPayload[],
  options: {
    excludeFields?: string[];
    maxSampleRows?: number;
  } = {}
): InferredSchema {
  const { excludeFields = [], maxSampleRows = 1000 } = options;

  if (data.length === 0) {
    throw new Error('Cannot infer schema from empty data');
  }

  // Sample data for analysis
  const sampleData = data.slice(0, maxSampleRows);
  const columns = Object.keys(data[0]).filter((c) => !excludeFields.includes(c));

  // Analyze each column
  const columnStats: ColumnStats[] = columns.map((col) =>
    analyzeColumn(
      col,
      sampleData.map((row) => row[col] ?? null)
    )
  );

  // Infer field types and build schema
  const fields: Record<string, FieldConfig> = {};
  const inferenceDetails: InferredSchema['inferenceDetails'] = [];

  for (const stats of columnStats) {
    const { type, distance, reason } = inferFieldType(stats);
    const weight = calculateWeight(stats.name, stats, type, columnStats);

    fields[stats.name] = {
      type,
      weight,
      distance,
    };

    inferenceDetails.push({
      field: stats.name,
      inferredType: type,
      uniqueValues: stats.uniqueCount,
      nullCount: stats.nullCount,
      reason,
    });
  }

  // Normalize weights
  const normalizedFields = normalizeWeights(fields);

  return {
    fields: normalizedFields,
    sampleData: sampleData.slice(0, 5),
    totalRows: data.length,
    inferenceDetails,
  };
}

/**
 * Parse CSV string into array of objects
 */
export function parseCSV(csvContent: string): PointPayload[] {
  const lines = csvContent.trim().split('\n');
  if (lines.length < 2) {
    throw new Error('CSV must have at least a header and one data row');
  }

  // Parse header
  const headers = parseCSVLine(lines[0]);
  
  // Parse data rows
  const data: PointPayload[] = [];
  for (let i = 1; i < lines.length; i++) {
    const values = parseCSVLine(lines[i]);
    if (values.length !== headers.length) continue;

    const row: PointPayload = {};
    for (let j = 0; j < headers.length; j++) {
      const header = headers[j].trim();
      let value: string | number | boolean | null = values[j].trim();

      // Try to parse as number
      if (value !== '' && !isNaN(Number(value))) {
        value = Number(value);
      }
      // Try to parse as boolean
      else if (value.toLowerCase() === 'true') {
        value = true;
      } else if (value.toLowerCase() === 'false') {
        value = false;
      }
      // Handle empty as null
      else if (value === '') {
        value = null;
      }

      row[header] = value;
    }
    data.push(row);
  }

  return data;
}

/**
 * Parse a single CSV line handling quoted values
 */
function parseCSVLine(line: string): string[] {
  const result: string[] = [];
  let current = '';
  let inQuotes = false;

  for (let i = 0; i < line.length; i++) {
    const char = line[i];
    const nextChar = line[i + 1];

    if (char === '"') {
      if (inQuotes && nextChar === '"') {
        current += '"';
        i++;
      } else {
        inQuotes = !inQuotes;
      }
    } else if (char === ',' && !inQuotes) {
      result.push(current);
      current = '';
    } else {
      current += char;
    }
  }
  result.push(current);

  return result;
}

/**
 * Create a human-readable schema summary
 */
export function schemaToSummary(schema: SimilaritySchema): string {
  const fields = Object.entries(schema.fields)
    .sort((a, b) => b[1].weight - a[1].weight)
    .map(
      ([name, config]) =>
        `  - ${name}: ${config.type} (weight: ${config.weight}, distance: ${config.distance || 'default'})`
    );

  return `Similarity Contract:\n${fields.join('\n')}`;
}
