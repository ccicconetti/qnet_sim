#!/bin/bash

#
# Check requirements
#

sim_exec="../../target/release/qnet_ll_sim"
executables="$sim_exec"
regular_files="conf.json.template"

for executable in $executables ; do 
    if [ ! -x $executable ] ; then
        echo "cannot find executable in current directory: $executable"
        exit 1
    fi
done

for regular_file in $regular_files ; do 
    if [ ! -r $regular_file ] ; then
        echo "cannot find file expected in current directory: $regular_file"
        exit 1
    fi
done

#
# Configuration
#

if [ "$DURATION" == "" ] ; then
    DURATION=60
fi
if [ "$SEED_INIT" == "" ] ; then
    SEED_INIT=0
fi
if [ "$SEED_END" == "" ] ; then
    SEED_END=24
fi

num_pairs_v="1 5 10 20"

#
# Execute experiments
#

if [[ "$DRY" == "" &&  -d "data" && ! -z "$( ls -A 'data/' )" ]] ; then
    read -p "directory 'data' exists and is non-empty: do you want to remove the content? [Y/N]: " confirm && [[ $confirm == [yY] || $confirm == [yY][eE][sS] ]] || exit 1
    rm -rf data/* 2> /dev/null
fi

rm conf.json 2> /dev/null

for NUM_PAIRS in $num_pairs_v ; do
    echo "# NUM_PAIRS $NUM_PAIRS"

    export DURATION NUM_PAIRS
    envsubst < conf.json.template > conf.json

    cmd="$sim_exec --append \
        --save-config \
        --additional-fields $NUM_PAIRS \
        --additional-header num_pairs \
        --seed-init $SEED_INIT --seed-end $SEED_END"


    if [ "$DRY" != "" ] ; then
        echo $cmd
    else
        eval $cmd
    fi

done

rm conf.json 2> /dev/null