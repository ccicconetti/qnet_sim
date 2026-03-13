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
    DURATION=300
fi
if [ "$SEED_INIT" == "" ] ; then
    SEED_INIT=0
fi
if [ "$SEED_END" == "" ] ; then
    SEED_END=100
fi

num_repeaters_v="3"
num_qubits_v="5 10 20 50 100 150"
prob_local_complete_v="0.5 0.6 0.7 0.8 0.9 0.95 0.98 0.99 0.999"
create_path_v="false"

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

for num_qubits in $num_qubits_v ; do
for NUM_REPEATERS in $num_repeaters_v ; do
for PROB_LOCAL_COMPLETE in $prob_local_complete_v ; do
for CREATE_PATH in $create_path_v ; do

    RATE=$(echo "scale=6; 100 / $num_qubits" | bc -l | sed 's/^\./0./')

    echo "# num_repeaters $NUM_REPEATERS, prob_local_complete $PROB_LOCAL_COMPLETE, create_path $CREATE_PATH, rate $RATE"

    export DURATION NUM_REPEATERS PROB_LOCAL_COMPLETE CREATE_PATH RATE
    envsubst < conf.json.template > conf.json

    cmd="./qnet_ll_sim --mini \
        --save-config \
        --append \
        --additional-fields $PROB_LOCAL_COMPLETE,$CREATE_PATH,$num_qubits \
        --additional-header prob_local_complete,create_path,num_qubits \
        --seed-init $SEED_INIT --seed-end $SEED_END"

    if [ "$DRY" != "" ] ; then
        echo $cmd
    else
        eval $cmd
    fi

done
done
done
done

rm conf.json 2> /dev/null
