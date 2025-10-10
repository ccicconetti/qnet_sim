#!/bin/bash

pushd .. > /dev/null
experiment=$(basename $PWD)
popd > /dev/null
sub=$(basename $PWD)

if [ "$PRINT_ONLY" != "" ] ; then
  echo $experiment-$sub.tgz
  exit 0
fi

wget http://turig.iit.cnr.it/~claudio/public/qnet_ll_sim/$experiment-$sub.tgz -O- | tar zx
