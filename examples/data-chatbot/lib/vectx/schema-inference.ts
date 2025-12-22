/**
 * CSV Parsing for vectX
 * 
 * Simple CSV parsing utility for importing tabular data.
 */

import type { PointPayload } from './types';

/**
 * Parse CSV content into an array of objects
 */
export function parseCSV(csvContent: string): PointPayload[] {
  const lines = csvContent.trim().split('\n');
  if (lines.length < 2) {
    return [];
  }

  // Parse header
  const headers = lines[0].split(',').map(h => h.trim().replace(/^"|"$/g, ''));
  
  // Parse rows
  const rows: PointPayload[] = [];
  for (let i = 1; i < lines.length; i++) {
    const values = lines[i].split(',').map(v => v.trim().replace(/^"|"$/g, ''));
    if (values.length !== headers.length) {
      continue; // Skip malformed rows
    }
    
    const row: PointPayload = {};
    for (let j = 0; j < headers.length; j++) {
      const value = values[j];
      // Try to parse as number
      const numValue = Number(value);
      if (!isNaN(numValue) && value !== '') {
        row[headers[j]] = numValue;
      } else if (value.toLowerCase() === 'true') {
        row[headers[j]] = true;
      } else if (value.toLowerCase() === 'false') {
        row[headers[j]] = false;
      } else {
        row[headers[j]] = value || null;
      }
    }
    rows.push(row);
  }
  
  return rows;
}
