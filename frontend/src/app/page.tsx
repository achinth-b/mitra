'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { BRAND } from '@/lib/brand';

export default function HomePage() {
  const router = useRouter();
  const { user, checkAuth, silentCheckAuth, isLoading, isInitialized, login, error } = useAuthStore();
  const [showLogin, setShowLogin] = useState(false);
  const [email, setEmail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [loginStatus, setLoginStatus] = useState<string | null>(null);

  const isMockMode = !process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY ||
    process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY.includes('YOUR_KEY_HERE');

  useEffect(() => {
    if (!isInitialized) {
      checkAuth();
    }
  }, [checkAuth, isInitialized]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        silentCheckAuth();
      }
    };
    const handleFocus = () => silentCheckAuth();

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleFocus);

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
    };
  }, [silentCheckAuth]);

  useEffect(() => {
    if (user.isLoggedIn && isInitialized) {
      router.push('/dashboard');
    }
  }, [user.isLoggedIn, isInitialized, router]);

  const pollForAuth = useCallback(async () => {
    if (isMockMode) return;
    const interval = setInterval(() => silentCheckAuth(), 2000);
    setTimeout(() => {
      clearInterval(interval);
      if (!user.isLoggedIn) {
        setLoginStatus(null);
        setIsSubmitting(false);
      }
    }, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, [silentCheckAuth, isMockMode, user.isLoggedIn]);

  const handleEmailLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) return;

    setIsSubmitting(true);

    if (isMockMode) {
      setLoginStatus('logging in...');
      const success = await login(email);
      if (success) {
        setLoginStatus('success!');
        setTimeout(() => router.push('/dashboard'), 500);
      } else {
        setLoginStatus(null);
        setIsSubmitting(false);
      }
    } else {
      setLoginStatus('sending magic link...');
      pollForAuth();
      const success = await login(email);
      if (success) {
        setLoginStatus('success!');
        setTimeout(() => router.push('/dashboard'), 500);
      } else if (document.visibilityState === 'visible') {
        setLoginStatus('check your email');
      }
    }
  };

  if (isLoading && !isInitialized) {
    return (
      <>
        <div className="container">
          <div className="content">
            <p className="coming-soon">loading...</p>
          </div>
        </div>
        <style jsx global>{styles}</style>
      </>
    );
  }

  return (
    <>
      <div className="container">
        <div className="content">
          {!showLogin ? (
            <>
              <h1>{BRAND.tagline}</h1>
              <h2>this <em>might</em> ruin your friendships.</h2>
              <p className="coming-soon" onClick={() => setShowLogin(true)} style={{ cursor: 'pointer' }}>
                enter →
              </p>
            </>
          ) : (
            <form onSubmit={handleEmailLogin} className="login-form">
              <h1>sign in</h1>

              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="your email"
                className="email-input"
                autoFocus
                disabled={isSubmitting}
              />

              {error && <p className="error-text">{error}</p>}

              {loginStatus ? (
                <p className="coming-soon">{loginStatus}</p>
              ) : (
                <button type="submit" disabled={!email || isSubmitting} className="coming-soon submit-btn">
                  continue →
                </button>
              )}

              <p className="hint">
                {isMockMode
                  ? 'dev mode — instant login'
                  : isSubmitting
                    ? 'click the link in your email'
                    : 'we\'ll send you a magic link'
                }
              </p>

              <p className="hint-small">a solana wallet will be created for you</p>

              {!isSubmitting ? (
                <button
                  type="button"
                  onClick={() => { setShowLogin(false); setLoginStatus(null); setEmail(''); }}
                  className="back-link"
                >
                  ← back
                </button>
              ) : !isMockMode && (
                <button
                  type="button"
                  onClick={() => { setIsSubmitting(false); setLoginStatus(null); }}
                  className="back-link"
                >
                  cancel
                </button>
              )}
            </form>
          )}
        </div>
      </div>
      <style jsx global>{styles}</style>
    </>
  );
}

const styles = `
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  html {
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  body {
    background-color: #000000;
    color: #ffffff;
    font-family: inherit;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    height: 100vh;
    height: 100dvh;
    width: 100vw;
    text-align: center;
    padding: 5vw;
    overflow: hidden;
  }

  .container {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    height: 100vh;
    height: 100dvh;
    width: 100vw;
    text-align: center;
    padding: 5vw;
  }

  .content {
    max-width: 90vw;
    width: 100%;
  }

  h1 {
    font-size: clamp(1.2rem, 5vw, 2rem);
    font-weight: normal;
    margin-bottom: clamp(1.5rem, 5vw, 3rem);
    line-height: 1.2;
  }

  h2 {
    font-size: clamp(1.2rem, 5vw, 2rem);
    font-weight: normal;
    margin-bottom: clamp(1.5rem, 5vw, 3rem);
    line-height: 1.2;
  }

  .coming-soon {
    font-size: clamp(1.2rem, 5vw, 2rem);
    font-style: italic;
    margin-top: clamp(2rem, 6vw, 4rem);
    opacity: 0.9;
  }

  .submit-btn {
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    font-family: inherit;
  }

  .submit-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .login-form {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .email-input {
    font-size: clamp(1rem, 4vw, 1.5rem);
    font-family: inherit;
    padding: clamp(0.5rem, 2vw, 1rem);
    width: 100%;
    max-width: 400px;
    text-align: center;
    background: transparent;
    border: none;
    border-bottom: 1px solid rgba(255, 255, 255, 0.3);
    color: white;
    margin-bottom: 0;
  }

  .email-input:focus {
    outline: none;
    border-bottom-color: rgba(255, 255, 255, 0.6);
  }

  .email-input::placeholder {
    color: rgba(255, 255, 255, 0.4);
    font-style: italic;
  }

  .error-text {
    font-size: clamp(0.9rem, 3vw, 1.1rem);
    color: #ff6b6b;
    margin-top: clamp(1rem, 3vw, 1.5rem);
  }

  .hint {
    font-size: clamp(0.9rem, 3vw, 1.1rem);
    opacity: 0.6;
    margin-top: clamp(2rem, 5vw, 3rem);
  }

  .hint-small {
    font-size: clamp(0.8rem, 2.5vw, 1rem);
    opacity: 0.4;
    margin-top: clamp(0.5rem, 1.5vw, 1rem);
  }

  .back-link {
    font-size: clamp(0.9rem, 3vw, 1.1rem);
    background: none;
    border: none;
    color: white;
    opacity: 0.4;
    cursor: pointer;
    margin-top: clamp(2rem, 5vw, 3rem);
    font-family: inherit;
  }

  .back-link:hover {
    opacity: 0.7;
  }

  /* Tablet specific adjustments */
  @media (min-width: 768px) and (max-width: 1024px) {
    h1, h2 {
      font-size: clamp(1.5rem, 4vw, 1.8rem);
    }

    .coming-soon {
      font-size: clamp(1.5rem, 4vw, 1.8rem);
    }
  }

  /* Mobile specific adjustments */
  @media (max-width: 767px) {
    body, .container {
      padding: 4vw;
    }

    h1, h2 {
      font-size: clamp(1.2rem, 6vw, 1.6rem);
    }

    .coming-soon {
      font-size: clamp(1.2rem, 6vw, 1.6rem);
    }
  }
`;
