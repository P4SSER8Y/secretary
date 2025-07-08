#!/usr/bin/env sh

function get_target() {
    items=("aarch64-unknown-linux-musl" "x86_64-unknown-linux-gnu" "aarch64-apple-darwin")
    if [ -z "$1" ]; then
        PS="select target"
        select target in "${items[@]}"
        do
            if [ -z "$target" ]; then
                return 2
            fi
            break
        done
    else
        if [[ $1 =~ ^[0-9]+$ ]]; then
            if [ $1 -lt 1 -o $1 -gt ${#items[*]} ]; then
                return 2
            else
                target=${items[$1]}
            fi
        else
            target=$1
        fi
    fi
    echo $target
    return 0
}
