#!/usr/bin/env bash
set -euo pipefail

BUCKET="${CFA_BANK_S3_BUCKET:-cfa-bank-local}"
ENDPOINT="${CFA_BANK_S3_ENDPOINT:-http://localhost:9000}"

echo "Starting MinIO via docker-compose..."
docker-compose up -d minio

echo "Waiting for MinIO to become healthy..."
until docker-compose exec -T minio curl -sf http://localhost:9000/minio/health/live > /dev/null; do
  sleep 1
done

echo "MinIO is live at $ENDPOINT"
echo "Console:    http://localhost:9001"
echo "Access key: minioadmin"
echo "Secret key: minioadmin"

# Create bucket via the mc client running inside the MinIO container
docker-compose exec -T minio sh -c "
  mc alias set local http://localhost:9000 minioadmin minioadmin > /dev/null
  mc mb --ignore-existing local/$BUCKET > /dev/null
  mc ls local/
"

echo ""
echo "Bucket '$BUCKET' is ready. Set these env vars for local development:"
echo "  export CFA_BANK_BACKEND=s3"
echo "  export CFA_BANK_S3_ENDPOINT=$ENDPOINT"
echo "  export CFA_BANK_S3_BUCKET=$BUCKET"
echo "  export AWS_ACCESS_KEY_ID=minioadmin"
echo "  export AWS_SECRET_ACCESS_KEY=minioadmin"
echo "  export AWS_REGION=us-east-1"
