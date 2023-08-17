package main

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

var StartCommand = &cobra.Command{
	Use:   "start",
	Short: "Start csync as daemon",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}

		return d.Start(func() error {
			clip, err := NewClipboard()
			if err != nil {
				return err
			}
			clip.Run()
			return nil
		})
	},
}

var StopCommand = &cobra.Command{
	Use:   "stop",
	Short: "Stop csync daemon",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}
		return d.Stop()
	},
}

var StatusCommand = &cobra.Command{
	Use:   "status",
	Short: "Show csync daemon status",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}
		return d.ShowStatus()
	},
}

var LogsCommand = &cobra.Command{
	Use:   "logs",
	Short: "Show csync daemon logs",

	DisableFlagParsing: true,

	RunE: func(_ *cobra.Command, args []string) error {
		d, err := NewDaemon()
		if err != nil {
			return err
		}
		path := d.LogPath()
		args = append(args, path)
		cmd := exec.Command("tail", args...)
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		cmd.Stdin = os.Stdin
		return cmd.Run()
	},
}

func ErrorExit(err error) {
	fmt.Printf("%s: %v\n", color.RedString("error"), err)
	os.Exit(1)
}
