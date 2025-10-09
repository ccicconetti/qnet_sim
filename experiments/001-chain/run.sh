#!/bin/bash

#
# Check requirements
#

executables="qnet_ll_sim"
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

num_repeaters_v="1 2 3 4 5"
memory_qubits_v="20 100"
num_pairs_v="1 10"

#
# Execute experiments
#

rm conf.json 2> /dev/null

for NUM_REPEATERS in $num_repeaters_v ; do
for MEMORY_QUBITS in $memory_qubits_v ; do
for num_pairs in $num_pairs_v ; do

    echo "# num_repeaters $NUM_REPEATERS, memory_qubits $MEMORY_QUBITS, num_pairs $num_pairs"

    PAIRS="[0,1]"
    for (( i = 1 ; i < $num_pairs ; i++ )) ; do
        if [ $(( i % 2)) -eq 0 ] ; then
            PAIRS="$PAIRS,[0,1]"
        else
            PAIRS="$PAIRS,[1,0]"
        fi
    done

    export DURATION NUM_REPEATERS MEMORY_QUBITS PAIRS
    envsubst < conf.json.template > conf.json

    cmd="./qnet_ll_sim --append \
        --save-config \
        --additional-fields $num_pairs \
        --additional-header num_pairs \
        --seed-init $SEED_INIT --seed-end $SEED_END"


    if [ "$DRY" != "" ] ; then
        echo $cmd
    else
        eval $cmd
    fi

done
done
done

rm conf.json 2> /dev/null