#!/bin/bash

GOOS="$1"
GOARCH="$2"
OUTPUT="$3"

GIT_DESC=$(git describe --tags)
GIT_TAG=$(git describe --tags --abbrev=0)
GIT_COMMIT=$(git rev-parse HEAD)
GIT_COMMIT_SHORT=$(git rev-parse --short HEAD)

if [[ "$GIT_DESC" == "$GIT_TAG" ]]; then
	BUILD_TYPE="stable"
	BUILD_VERSION="$GIT_TAG"
else
	BUILD_TYPE="dev"
	BUILD_VERSION="${GIT_TAG}-dev_${GIT_COMMIT_SHORT}"
fi

if git status --porcelain | grep -E '(M|A|D|R|\?)' > /dev/null; then
	BUILD_TYPE="dev-uncommitted"
	BUILD_VERSION="${BUILD_VERSION}-uncommitted"
fi

if [[ -z $OUTPUT ]]; then
	OUTPUT="bin/csync"
fi

cat << EOF
Build Args:
GOOS=${GOOS}
GOARCH=${GOARCH}
OUTPUT=${OUTPUT}
GIT_DESC=${GIT_DESC}
GIT_TAG=${GIT_TAG}
GIT_COMMIT=${GIT_COMMIT}
GIT_COMMIT_SHORT=${GIT_COMMIT_SHORT}
BUILD_TYPE=${BUILD_TYPE}
BUILD_VERSION=${BUILD_VERSION}
EOF

echo ""
echo "Building..."
CGO_ENABLED=1 GOOS=${GOOS} GOARCH=${GOARCH} go build -ldflags "-X main.Version=${BUILD_VERSION} -X main.BuildType=${BUILD_TYPE} -X main.BuildCommit=${GIT_COMMIT} -X main.BuildTime=$(date +%F-%Z/%T)" -o ${OUTPUT}
if [[ $? -ne 0 ]]; then
	echo "Build failed"
	exit 1
fi
echo "Build to ${OUTPUT} done"
