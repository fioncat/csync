package main

import (
	"archive/tar"
	"compress/gzip"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"time"
)

const (
	getLatestVersionURL     = "https://api.github.com/repos/fioncat/csync/releases/latest"
	getLatestVersionTimeout = time.Second * 5

	downloadReleaseURL     = "https://github.com/fioncat/csync/releases/download/%s/csync_%s.tar.gz"
	downloadReleaseTimeout = time.Minute
)

type githubRelease struct {
	Name string `json:"name"`
}

func GetLatestVersion() (string, error) {
	req, err := http.NewRequest("GET", getLatestVersionURL, http.NoBody)
	if err != nil {
		return "", fmt.Errorf("Create http request: %w", err)
	}

	client := http.Client{Timeout: getLatestVersionTimeout}

	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("Do http request: %w", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("Read http response body: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		if len(data) == 0 {
			return "", fmt.Errorf("Request github return bad status: %s", resp.Status)
		}
		return "", fmt.Errorf("Request github error: %s", string(data))
	}

	var release githubRelease
	err = json.Unmarshal(data, &release)
	if err != nil {
		return "", fmt.Errorf("Decode response body to json: %w", err)
	}

	if release.Name == "" {
		return "", fmt.Errorf("The latest release returned by github does not have a name")
	}

	return release.Name, nil
}

func DownloadRelease(version, target, path string) error {
	url := fmt.Sprintf(downloadReleaseURL, version, target)
	req, err := http.NewRequest("GET", url, http.NoBody)
	if err != nil {
		return fmt.Errorf("Create http request: %w", err)
	}

	client := http.Client{Timeout: downloadReleaseTimeout}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("Do http request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		if resp.StatusCode == http.StatusNotFound {
			return fmt.Errorf("Could not find version %s for target %s", version, target)
		}
		return fmt.Errorf("Download release bad status: %s", resp.Status)
	}

	err = ensureDir(path)
	if err != nil {
		return fmt.Errorf("Ensure dir for %q: %w", path, err)
	}

	file, err := os.OpenFile(path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0644)
	if err != nil {
		return fmt.Errorf("Open download file: %w", err)
	}
	defer file.Close()

	_, err = io.Copy(file, resp.Body)
	if err != nil {
		return fmt.Errorf("Download data: %w", err)
	}

	return nil
}

func UnTarTo(src, dst string) error {
	file, err := os.Open(src)
	if err != nil {
		return fmt.Errorf("Open downloaded tar file: %w", err)
	}
	defer file.Close()

	gr, err := gzip.NewReader(file)
	if err != nil {
		return fmt.Errorf("Open gzip file: %w", err)
	}
	defer gr.Close()

	tr := tar.NewReader(gr)
	for {
		hdr, err := tr.Next()
		if err != nil {
			if errors.Is(err, io.EOF) {
				return nil
			}
			return fmt.Errorf("Read gzip file: %w", err)
		}
		if hdr == nil {
			continue
		}

		dstPath := filepath.Join(dst, hdr.Name)

		if hdr.Typeflag == tar.TypeReg {
			err = ensureDir(dstPath)
			if err != nil {
				return fmt.Errorf("Untar ensure dir: %w", err)
			}
			dstFile, err := os.OpenFile(dstPath, os.O_CREATE|os.O_RDWR, os.FileMode(hdr.Mode))
			if err != nil {
				return fmt.Errorf("Untar open file: %w", err)
			}

			_, err = io.Copy(dstFile, tr)
			if err != nil {
				return fmt.Errorf("Untar copy file: %w", err)
			}

			err = dstFile.Close()
			if err != nil {
				return fmt.Errorf("Untar close file: %w", err)
			}
		}
	}
}

func ReplaceBinary(dstPath, srcPath string) error {
	// TODO: support Windows system
	dstDir := filepath.Dir(dstPath)

	testPath := filepath.Join(dstDir, "test.txt")
	checkPermissionCmd := exec.Command("touch", testPath)

	err := checkPermissionCmd.Run()
	requirePermission := err != nil
	if !requirePermission {
		err = os.Remove(testPath)
		if err != nil {
			return fmt.Errorf("Remove tmp test file: %w", err)
		}
	}

	var mvCmd *exec.Cmd
	if requirePermission {
		ExecInfo("Escalated permissions are required, please input sudo password")
		mvCmd = exec.Command("sudo", "mv", srcPath, dstPath)
	} else {
		mvCmd = exec.Command("mv", srcPath, dstPath)
	}
	mvCmd.Stdout = os.Stdout
	mvCmd.Stderr = os.Stderr
	mvCmd.Stdin = os.Stdin

	err = mvCmd.Run()
	if err != nil {
		return fmt.Errorf("Execute mv command: %w", err)
	}

	return nil
}
