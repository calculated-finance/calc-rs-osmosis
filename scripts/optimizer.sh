#!/bin/bash
if [ $# -gt 0 ] && [ $1 = "--tag-testnet" ]
then
  YEAR=$(date '+%y')
  MONTH=$(date '+%m')
  DAY=$(date '+%d')
  echo "generating build hash..."
  OUTPUT=$(docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/workspace-optimizer:0.12.9)

  regex='(.{65}) dca.wasm'

  if [[ $OUTPUT =~ $regex ]]
  then
    BUILD_HASH=$(echo $BASH_REMATCH[1] | cut -c 1-7)
    echo "build hash: $BUILD_HASH"

    echo "updating local tags..."
    git fetch --all --tags
    ALL_TAGS=$(git tag)
    ALL_TAGS_ARRAY=($ALL_TAGS)
    LATEST_TAG=${ALL_TAGS_ARRAY[-1]}
    echo $LATEST_TAG

    # match vX.X.X-rc.X
    REGEX_WITH_RC='(v[0-9]+\.[0-9]+\.[0-9]+)-rc\.([0-9]+)'

    # match vX.X.X
    REGEX_WITHOUT_RC='(v[0-9]+\.[0-9]+\.[0-9]+)'

    if [[ $LATEST_TAG =~ $REGEX_WITH_RC ]]
    then
      echo "existing rc tag found for latest version, incrementing..."
      LATEST_VERSION=${BASH_REMATCH[1]}
      LATEST_RC=${BASH_REMATCH[2]}
      LATEST_RC_PLUS_ONE=$(($LATEST_RC + 1))
      TAG=$LATEST_VERSION-rc.$LATEST_RC_PLUS_ONE+$BUILD_HASH
      echo "latest version: $LATEST_VERSION"
      echo "latest rc: $LATEST_RC"
      echo "new tag: $TAG"
      git tag -a $TAG -m "testnet $DAY.$MONTH.$YEAR"
      git push origin $TAG

    elif [[ $LATEST_TAG =~ $REGEX_WITHOUT_RC ]]
    then
      echo "no rc tags found for latest version, creating one..."
      LATEST_VERSION=${BASH_REMATCH[1]}
      TAG=$LATEST_VERSION-rc.1+$BUILD_HASH
      echo "latest version: $LATEST_VERSION"
      echo "new tag: $TAG"
      git tag -a $TAG -m "testnet $DAY.$MONTH.$YEAR"
      git push origin $TAG
      
    else
      echo "failed to match any versions"
    fi
  fi

else
  docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.9
fi