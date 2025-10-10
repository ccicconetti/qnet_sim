#!/bin/bash

script=../../scripts/download-artifacts.sh

if [ ! -x $script ] ; then
  echo "could not find download-artifacts.sh in ../../"
  exit 1
fi

tar zcf $(PRINT_ONLY=1 $script) data