#!/usr/bin/env sh

function get_target() {
    items=("aarch64-unknown-linux-musl" "x86_64-unknown-linux-gnu")
    if [ -z "$1" ]; then
        PS="select target"
        select target in "${items[@]}"
        do
            if [ -z "$target" ]; then
                return 0
            fi
            break
        done
    else
        if [[ $1 =~ ^[0-9]+$ ]]; then
            if [ $1 -lt 1 -o $1 -gt ${#items[*]} ]; then
                return 0
            else
                target=${items[$1-1]}
            fi
        else
            target=$1
        fi
    fi
    echo $target
    return 0
}