#!/bin/bash

directory_name=${PWD##*/}
tmp_path="/mnt/c/Windows/temp/$directory_name"

mkdir -p $tmp_path

rsync . $tmp_path -r --exclude-from=.gitignore

cd $tmp_path


while getopts ":r" opt; do
  case $opt in
    r)
      powershell.exe -Command "cargo run $@"
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      ;;
    :)
      powershell.exe -Command "cargo build $@"
  esac
done