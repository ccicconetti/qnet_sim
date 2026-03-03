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
    SEED_END=10
fi

num_repeaters_v="7"
num_pairs_v="1 2 5 10 20 30 40 50"
memory_qubits_v="10 20 40 60 80 100 200 300"

#
# Execute experiments
#

if [ "$OVERRIDE" == "" ] ; then
    if [[ "$DRY" == "" &&  -d "data" && ! -z "$( ls -A 'data/' )" ]] ; then
        read -p "directory 'data' exists and is non-empty: do you want to remove the content? [Y/N]: " confirm && [[ $confirm == [yY] || $confirm == [yY][eE][sS] ]] || exit 1
        rm -rf data/* 2> /dev/null
    fi
fi

rm conf.json 2> /dev/null

for NUM_REPEATERS in $num_repeaters_v ; do
for MEMORY_QUBITS in $memory_qubits_v ; do
for num_pairs in $num_pairs_v ; do

    echo "# num_repeaters $NUM_REPEATERS, memory_qubits $MEMORY_QUBITS, num_pairs $num_pairs,"

    OGS_MEMORY_QUBITS=$(( MEMORY_QUBITS / 2 ))

    PAIRS="[0,1]"
    for (( i = 1 ; i < $num_pairs ; i++ )) ; do
        PAIRS="$PAIRS,[0,1]"
    done

    export DURATION NUM_REPEATERS PAIRS MEMORY_QUBITS OGS_MEMORY_QUBITS
    envsubst < conf.json.template > conf.json

    cmd="./qnet_ll_sim --append \
        --additional-fields $num_pairs,$NUM_REPEATERS,$MEMORY_QUBITS,$DURATION \
        --additional-header num_pairs,num_repeaters,memory_qubits,duration \
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
