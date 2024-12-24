#!/bin/bash
# Generate the static js file

if ! type esbuild ; then
  echo "Install esbuild to run smpgc"
  exit 1
fi

ROOT="$(dirname $0)"

esbuild $ROOT/client/index.js --bundle --minify --sourcemap --outfile=$ROOT/www/static/v1.js || exit 1

if [[ "$1" == "--git-add" ]] ; then
  echo "Adding generated files to git"
  git add $ROOT/www/static/v1.js $ROOT/www/static/v1.css
fi
echo "done"
