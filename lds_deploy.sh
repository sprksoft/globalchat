#!/bin/bash
set -e
RUST_PROJECTS=( lmetrics profanity smppgc ) # NOTE: also change in prod_images.sh if project is an image
PROD_SERVER="ldev@192.168.1.69"
PROJECT="smppserver"

# software override
SSH="ssh"

ALLOW_DEPLOY_ON_MAIN="false"
ALLOW_UNCOMMITED="false"

if [[ "$@" == *"--allow-deploy-on-main"* ]] ; then
  ALLOW_DEPLOY_ON_MAIN="true"
fi
if [[ "$@" == *"--allow-uncommited"* ]] ; then
  ALLOW_UNCOMMITED="true"
fi

year=$(date --utc +'%-Y')
month=$(date --utc +'%-m')

new_ver="$year.$month.0"
if git tag | rg "^$year\.$month\." > /dev/null ; then
  new_ver="$year.$month.$(($(git tag | rg -r '$1' "^$year\.$month\.([0-9]*)" | tail -n 1)+1))"
fi

if [[ "$(git status -s)" != "" ]] && [[ "$ALLOW_UNCOMMITED" == "false" ]] ; then
  echo "Uncommitted changes"
  exit 1
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "dev" ]] && [[ "$ALLOW_DEPLOY_ON_MAIN" == "false" ]] ; then
  echo "Not on dev branch"
  exit 1
fi
echo "new version: $new_ver"

for rust_proj in ${RUST_PROJECTS[@]} ; do
  echo "Bumping Cargo.toml of $rust_proj..."
  new_cargo_toml=$(cat $rust_proj/Cargo.toml | rg --passthru '^version\s*=\s*"([0-9]*\.[0-9]*\.[0-9]*)"' -r "version=\"$new_ver\"")
  echo "$new_cargo_toml" > $rust_proj/Cargo.toml
done

if [[ $@ == *"--no-git"* ]] ; then
  echo "skipping git commands"
  exit 0
fi

echo "Checking into version control..."
git add .
git commit -m "bump: to v$new_ver"

git tag -a $new_ver -m "v$new_ver"

echo "Pushing to remote"
git push
git push --tags

if [[ "$BRANCH" != "main" ]] ; then
  echo "Merging into main branch..."
  git checkout main
  git merge dev
  git push
  git push --tags
  git checkout dev
else
  echo "No need to merge into main branch"
fi

echo "Building prod images"
./prod_images.sh build

echo "Pushing prod images..."
./prod_images.sh push "$PROD_SERVER"

echo "redeploying containers on prod server..."
$SSH $PROD_SERVER "~/source/repos/ldeveuorg-infra/deploy.sh"

echo "done"
