package main

import (
	"fmt"
	"runtime"
	"strings"

	"github.com/spf13/cobra"
)

var (
	Version     = "<unknown>"
	BuildType   = "<unknown>"
	BuildCommit = "<unknown>"
	BuildTime   = "<unknown>"
)

var Command = &cobra.Command{
	Use:   "csync",
	Short: "Start csync",

	Version: Version,

	SilenceErrors: true,
	SilenceUsage:  true,

	CompletionOptions: cobra.CompletionOptions{
		HiddenDefaultCmd: true,
	},

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		clip, err := NewClipboard()
		if err != nil {
			return err
		}
		clip.Run()
		return nil
	},
}

var VersionCommand = &cobra.Command{
	Use:   "version",
	Short: "Show csync full version info",

	Args: cobra.ExactArgs(0),

	RunE: func(_ *cobra.Command, _ []string) error {
		fmt.Printf("csync %s\n", Version)
		fmt.Printf("golang %s\n", strings.TrimPrefix(runtime.Version(), "go"))
		fmt.Println("")
		fmt.Printf("Build type:   %s\n", BuildType)
		fmt.Printf("Build target: %s-%s\n", runtime.GOOS, runtime.GOARCH)
		fmt.Printf("Commit SHA:   %s\n", BuildCommit)
		fmt.Printf("Build time:   %s\n", BuildTime)
		fmt.Println("")

		configPath, err := configPath()
		if err != nil {
			return err
		}

		metaDir, err := GetMetaDir()
		if err != nil {
			return err
		}

		fmt.Printf("Config path: %s\n", configPath)
		fmt.Printf("Meta path:   %s\n", metaDir)

		return nil
	},
}

func main() {
	Command.AddCommand(StartCommand, StopCommand, StatusCommand, LogsCommand)
	Command.AddCommand(SendCommand, RecvCommand)
	Command.AddCommand(VersionCommand, UpdateCommand)

	err := Command.Execute()
	if err != nil {
		ErrorExit(err)
	}
}
