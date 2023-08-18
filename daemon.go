package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/fatih/color"
	"github.com/sevlyar/go-daemon"
)

type Daemon struct {
	pid int

	pidPath string
	logPath string
}

func NewDaemon() (*Daemon, error) {
	localDir, err := GetMetaDir()
	if err != nil {
		return nil, fmt.Errorf("Get daemon dir error: %w", err)
	}

	pidPath := filepath.Join(localDir, "pid")
	logPath := filepath.Join(localDir, "logs")
	pid, err := getDaemonPid(pidPath)
	if err != nil {
		return nil, err
	}

	return &Daemon{
		pid:     pid,
		pidPath: pidPath,
		logPath: logPath,
	}, nil
}

func getDaemonPid(path string) (int, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return -1, nil
		}
		return 0, err
	}

	if len(data) == 0 {
		return -1, nil
	}

	str := strings.TrimSpace(string(data))
	pid, err := strconv.Atoi(str)
	if err != nil {
		fmt.Printf("WARN: invalid pid %s: %q\n", path, str)
		return -1, nil
	}
	return pid, nil
}

func (d *Daemon) Start(f func() error) error {
	dctx := &daemon.Context{
		PidFileName: d.pidPath,
		PidFilePerm: 0644,
		LogFileName: d.logPath,
		LogFilePerm: 0640,
		Umask:       027,
	}

	rd, err := dctx.Reborn()
	if err != nil {
		if err == daemon.ErrWouldBlock {
			return nil
		}
		return err
	}
	if rd != nil {
		return nil
	}
	defer dctx.Release()

	return f()
}

func (d *Daemon) GetProcess() (*os.Process, error) {
	if d.pid < 0 {
		return nil, nil
	}
	return os.FindProcess(d.pid)
}

func (d *Daemon) Stop() error {
	process, err := d.GetProcess()
	if err != nil {
		return err
	}
	if process == nil {
		return nil
	}
	if isDaemonRunning(process) {
		fmt.Printf("killing %d...\n", d.pid)
		err = process.Kill()
		if err != nil {
			return fmt.Errorf("Kill process: %v", err)
		}
		time.Sleep(time.Second * 2)
		if isDaemonRunning(process) {
			return fmt.Errorf("Process is still running after killing, please try to kill it manually")
		}
	}
	return os.Remove(d.pidPath)
}

func (d *Daemon) ShowStatus() error {
	process, err := d.GetProcess()
	if err != nil {
		return fmt.Errorf("Get process: %v", err)
	}
	if process == nil {
		fmt.Println("csync dead")
		return nil
	}
	if isDaemonRunning(process) {
		attr := color.New(color.FgGreen, color.Bold)
		status := attr.Sprint("running")
		fmt.Printf("csync %d, %s\n", d.pid, status)
		return nil
	}
	attr := color.New(color.FgRed, color.Bold)
	status := attr.Sprint("not running")
	fmt.Printf("csync %d, %s\n", d.pid, status)

	return nil
}

func (d *Daemon) Restart(f func() error) error {
	err := d.Stop()
	if err != nil {
		return fmt.Errorf("Stop daemon: %s", err)
	}
	return d.Start(f)
}

func isDaemonRunning(p *os.Process) bool {
	err := p.Signal(syscall.Signal(0))
	return err == nil
}

func (d *Daemon) LogPath() string {
	return d.logPath
}
