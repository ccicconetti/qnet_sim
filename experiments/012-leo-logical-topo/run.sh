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

if [ "$SEED_INIT" == "" ] ; then
    SEED_INIT=0
fi
if [ "$SEED_END" == "" ] ; then
    SEED_END=200
fi

memory_qubits_v="10 100"
detectors_v="10 100"

#
# Execute experiments
#

if [[ "$DRY" == "" &&  -d "data" && ! -z "$( ls -A 'data/' )" ]] ; then
    read -p "directory 'data' exists and is non-empty: do you want to remove the content? [Y/N]: " confirm && [[ $confirm == [yY] || $confirm == [yY][eE][sS] ]] || exit 1
    rm -rf data/* 2> /dev/null
fi

rm conf.json 2> /dev/null

for (( snapshot = 0 ; snapshot < 500 ; snapshot++ )); do
    INPUT_PATH=input/simulation_data_161025/snapshots/activelinks_snap$snapshot.txt

for MEMORY_QUBITS in $memory_qubits_v ; do
for DETECTORS in $detectors_v ; do

    echo "# MEMORY_QUBITS $MEMORY_QUBITS DETECTORS $DETECTORS snapshot $snapshot"

    export DURATION MEMORY_QUBITS DETECTORS INPUT_PATH PAIRS
    envsubst < conf.json.template > conf.json

    cmd="$sim_exec --append \
        --additional-fields $MEMORY_QUBITS,$DETECTORS,$snapshot \
        --additional-header num_qubits,detectors,snapshot \
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
