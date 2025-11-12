from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Application settings with environment variable support."""
    
    # Environment
    environment: str = "development"
    
    # Database
    database_url: str
    
    # Redis
    redis_url: str
    
    # Auth
    secret_key: str
    jwt_algorithm: str = "HS256"
    access_token_expire_minutes: int = 10080  # 7 days
    
    # Email
    resend_api_key: str
    
    # Frontend
    frontend_url: str = "http://localhost:3000"
    
    # App Config
    initial_credit_balance: int = 100000  # 1000 credits (stored as cents)
    invite_bonus_credits: int = 50000  # 500 credits
    max_invite_bonuses: int = 5
    
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
    )


settings = Settings()

