/**
 * Mitra Brand Constants
 * 
 * These values should be used across all frontends (web app, front-page, etc.)
 * to ensure consistent branding.
 * 
 * Source of truth: This file. Update here and sync to other locations.
 */

export const BRAND = {
  /** Company name */
  name: 'mitra',
  
  /** Tagline - used everywhere */
  tagline: 'bet on (or against) your friends.',
  
  /** Meta description for SEO */
  description: 'bet on (or against) your friends.',
  
  /** App URLs */
  urls: {
    app: 'https://app.mitra.markets',
    landing: 'https://mitra.markets',
  },
} as const;
