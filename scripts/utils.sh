#!/usr/bin/env sh

function get_target() {
    items=("aarch64-unknown-linux-musl" "x86_64-unknown-linux-gnu")
    if [ -z "$1" ]; then
        PS="select target"
        select target in "${items[@]}"
        do
            if [ -z "$target" ]; then
                echo "unknown target"
                return 1
            fi
            break
        done
    else
        if [ $1 -lt 1 -o $1 -gt ${#items[*]} ]; then
            echo "index=$1 not valid"
            return 1
        fi
        target=${items[$1-1]}
    fi
    echo $target
    return 0
}