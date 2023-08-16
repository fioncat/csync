package main

import (
	"fmt"
	"os"

	"github.com/fatih/color"
)

func ErrorExit(err error) {
	fmt.Printf("%s: %v\n", color.RedString("error"), err)
	os.Exit(1)
}
