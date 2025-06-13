#!/bin/bash
set -e

echo "🔃 Run migrate database..."
DATABASE_URL=postgres://postgres:postgres@db:5432/app_db sea-orm-cli migrate up

echo "📦 Generate entity SeaORM..."
DATABASE_URL=postgres://postgres:postgres@db:5432/app_db sea-orm-cli generate entity -o src/entity -l