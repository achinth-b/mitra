/**
 * Mitra Brand Constants
 * 
 * These values should be used across all frontends (web app, front-page, etc.)
 * to ensure consistent branding.
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

// For environments that can't import TypeScript (like plain HTML)
// Use the JavaScript constants from this file or copy values directly
