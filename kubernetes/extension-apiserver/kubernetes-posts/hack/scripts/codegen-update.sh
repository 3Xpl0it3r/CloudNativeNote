#!/usr/bin/env bash

CURRENT_DIR=$(echo "$(pwd)/$line")
REPO_DIR="$CURRENT_DIR"
IMAGE_NAME="kubernetes-codegen:latest"


echo "Building codgen Docker image ...."
docker build -f "${CURRENT_DIR}/hack/docker/codegen.dockerfile" \
             -t "${IMAGE_NAME}" \
             "${REPO_DIR}" 
            

cmd0="go mod tidy && /go/src/k8s.io/code-generator/generate-groups.sh  all  \
        "github.com/Marcos30004347/kubernetes-posts/pkg/generated" \
        "github.com/Marcos30004347/kubernetes-posts/pkg/apis" \
        baz:v1alpha1 -h /go/src/k8s.io/code-generator/hack/boilerplate.go.txt"

cmd1="go mod tidy && /go/src/k8s.io/code-generator/generate-groups.sh  "deepcopy,defaulter,conversion,informer,listers,client" \
        "github.com/Marcos30004347/kubernetes-posts/pkg/generated" \
        "github.com/Marcos30004347/kubernetes-posts/pkg/apis" \
        "github.com/Marcos30004347/kubernetes-posts/pkg/apis" \
        baz:v1alpha1 -h /go/src/k8s.io/code-generator/hack/boilerplate.go.txt"

echo ${cmd0}
echo ${cmd1}
    
echo "Generating client codes ...."

echo docker run --rm -v "${REPO_DIR}:/go/src/github.com/Marcos30004347/kubernetes-posts" \
        "${IMAGE_NAME}" /bin/bash -c "${cmd0}"
echo docker run --rm -v "${REPO_DIR}:/go/src/github.com/Marcos30004347/kubernetes-posts" \
        "${IMAGE_NAME}" /bin/bash -c "${cmd1}"
