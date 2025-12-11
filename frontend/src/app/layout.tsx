import type { Metadata } from 'next';
import { EB_Garamond } from 'next/font/google';
import { BRAND } from '@/lib/brand';
import './globals.css';

const ebGaramond = EB_Garamond({
  subsets: ['latin'],
  weight: ['400', '500', '600'],
  style: ['normal', 'italic'],
  variable: '--font-serif',
});

import { ClientWalletProvider } from '@/components/WalletProvider';

export const metadata: Metadata = {
  title: BRAND.name,
  description: BRAND.description,
  icons: {
    icon: "data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22><text y=%22.9em%22 font-size=%2290%22>🤝</text></svg>",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={ebGaramond.className}>
      <body className={ebGaramond.className}>
        <ClientWalletProvider>
          {children}
        </ClientWalletProvider>
      </body>
    </html>
  );
}
