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
    SEED_END=16
fi

num_pairs_v="1 20 50"
memory_qubits_v="10 50 100"
alice=35
bob=36
pair="[ \"ogs#$alice\", \"ogs#$bob\" ]"

#
# Execute experiments
#

if [[ "$DRY" == "" &&  -d "data" && ! -z "$( ls -A 'data/' )" ]] ; then
    read -p "directory 'data' exists and is non-empty: do you want to remove the content? [Y/N]: " confirm && [[ $confirm == [yY] || $confirm == [yY][eE][sS] ]] || exit 1
    rm -rf data/* 2> /dev/null
fi

rm conf.json 2> /dev/null

for (( snapshot = 0 ; snapshot < 500 ; snapshot++ )); do
for MEMORY_QUBITS in $memory_qubits_v ; do
for num_pairs in $num_pairs_v ; do

    INPUT_PATH=input/simulation_data_161025/snapshots/activelinks_snap$snapshot.txt

    alice_present=$(grep $'\t'$alice$'\t' $INPUT_PATH)
    bob_present=$(grep $'\t'$bob$'\t' $INPUT_PATH)
    if [ "$alice_present" == "" ] || [ "$bob_present" == "" ] ; then
        echo "skipping snapshot $snapshot because one of the nodes is not present"
        continue
    fi

    echo "# MEMORY_QUBITS $MEMORY_QUBITS num_pairs $num_pairs snapshot $snapshot"

    PAIRS=$pair
    for (( i = 1 ; i < $num_pairs ; i++ )) ; do
        PAIRS="$PAIRS,$pair"
    done


    export DURATION MEMORY_QUBITS INPUT_PATH PAIRS
    envsubst < conf.json.template > conf.json

    cmd="$sim_exec --append \
        --save-config \
        --additional-fields $alice,$bob,$MEMORY_QUBITS,$num_pairs,$snapshot \
        --additional-header alice,bob,num_qubits,num_pairs,snapshot \
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
