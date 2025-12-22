# Docker Deployment Guide for vectX

## Prerequisites

1. Docker installed and running
2. Docker Hub account: `antonellofratepietro`
3. Docker Hub authentication token (for CI/CD)

## Manual Build and Push

### 1. Build the Docker Image

```bash
cd /Users/d695663/Desktop/Dev/rust/distx

# Build for local testing
docker build -t antonellofratepietro/vectx:latest .

# Or build with version tag
docker build -t antonellofratepietro/vectx:v0.2.7 .
```

### 2. Test the Image Locally

```bash
# Run the container
docker run -d --name vectx-test \
  -p 6333:6333 \
  -p 6334:6334 \
  -v vectx_storage:/qdrant/storage \
  antonellofratepietro/vectx:latest

# Check if it's running
curl http://localhost:6333/healthz

# View logs
docker logs vectx-test

# Stop and remove
docker stop vectx-test && docker rm vectx-test
```

### 3. Login to Docker Hub

```bash
docker login -u antonellofratepietro
# Enter your Docker Hub password or access token
```

### 4. Push to Docker Hub

```bash
# Push latest
docker push antonellofratepietro/vectx:latest

# Push version tag
docker push antonellofratepietro/vectx:v0.2.7

# Push multiple tags
docker tag antonellofratepietro/vectx:latest antonellofratepietro/vectx:v0.2.7
docker push antonellofratepietro/vectx:v0.2.7
docker push antonellofratepietro/vectx:latest
```

### 5. Multi-Architecture Build (Optional)

For AMD64 and ARM64 support:

```bash
# Create buildx builder
docker buildx create --name vectx-builder --use

# Build and push for multiple platforms
docker buildx build --platform linux/amd64,linux/arm64 \
  -t antonellofratepietro/vectx:latest \
  -t antonellofratepietro/vectx:v0.2.7 \
  --push .
```

## Using Makefile

The Makefile includes convenient commands:

```bash
# Build image
make docker-build

# Push to Docker Hub (requires login)
make docker-push

# Run locally
make docker-run

# View logs
make docker-logs

# Stop container
make docker-stop
```

## CI/CD Deployment

### GitHub Actions Setup

1. **Add Docker Hub Secrets to GitHub:**
   - Go to: Settings → Secrets and variables → Actions
   - Add secrets:
     - `DOCKERHUB_USERNAME`: `antonellofratepietro`
     - `DOCKERHUB_TOKEN`: Your Docker Hub access token

2. **Automatic Deployment:**
   - Push a tag starting with `v*` (e.g., `v0.2.7`)
   - The workflow will automatically:
     - Build the Docker image
     - Tag it with version, minor, major, and latest
     - Push to Docker Hub

3. **Manual Deployment:**
   - Go to Actions → "Docker Build and Push"
   - Click "Run workflow"
   - Optionally specify a tag

## Deleting Old Images from Docker Hub

**Note:** I cannot delete images from Docker Hub programmatically. You need to do this manually:

1. **Via Docker Hub Web UI:**
   - Go to https://hub.docker.com/r/antonellofratepietro/distx
   - Navigate to the image you want to delete
   - Click "Delete" (if you have permissions)

2. **Via Docker Hub API:**
   ```bash
   # Get your token
   TOKEN=$(curl -s -H "Content-Type: application/json" \
     -X POST \
     -d '{"username": "antonellofratepietro", "password": "YOUR_PASSWORD"}' \
     https://hub.docker.com/v2/users/login/ | jq -r .token)
   
   # Delete a tag
   curl -X DELETE \
     -H "Authorization: JWT ${TOKEN}" \
     https://hub.docker.com/v2/repositories/antonellofratepietro/distx/tags/TAG_NAME/
   ```

3. **Recommended Approach:**
   - Keep the old `distx` images for backward compatibility
   - Start using `vectx` images going forward
   - Deprecate `distx` images after a transition period

## Verification

After pushing, verify the image:

```bash
# Pull and test
docker pull antonellofratepietro/vectx:latest
docker run -p 6333:6333 antonellofratepietro/vectx:latest

# Check in browser
open http://localhost:6333/dashboard
```

## Image Tags Strategy

- `latest`: Always points to the most recent release
- `v0.2.7`: Specific version
- `v0.2`: Minor version (latest patch)
- `v0`: Major version (latest minor)

