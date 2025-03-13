# -*- coding: utf-8 -*-

from pathlib import Path

import json
import subprocess
import sys

from albert import *

md_iid = "3.0"
md_version = "2.0"
md_name = "Csync"
md_description = "Csync"
md_license = "MIT"
md_url = "https://github.com/fioncat/csync/tree/main/albert"
md_authors = ["@wenqian"]


class Plugin(PluginInstance, TriggerQueryHandler):

    def __init__(self):
        PluginInstance.__init__(self)
        TriggerQueryHandler.__init__(self)
        if sys.platform == "linux":
            self.control_cmd = "/usr/bin/csynctl"
        elif sys.platform == "darwin":
            self.control_cmd = "/opt/homebrew/bin/csynctl"
        else:
            self.control_cmd = "csynctl"

        self.icon_text = [f"file:{Path(__file__).parent}/text.png"]
        self.icon_image = [f"file:{Path(__file__).parent}/image.png"]
        self.icon_file = [f"file:{Path(__file__).parent}/file.png"]

    def defaultTrigger(self):
        return "cc "

    def handleTriggerQuery(self, query):
        if not query.isValid:
            return

        debug("Csync query: %s" % query.string)

        try:
            result = subprocess.run(
                [self.control_cmd, "get", "metadata", "-o", "json"],
                capture_output=True,
                text=True,
                check=True)

            resp = json.loads(result.stdout)
            items = resp["items"]

        except subprocess.CalledProcessError as e:
            error("Csync error: %s" % e)
            query.add(
                [StandardItem(id="csync-error", text="Csync command error")])
            return

        results = []
        for item in items:
            if query.string not in item["summary"]:
                continue
            debug(f"Csync item: {item}")

            action = Action("copy",
                            "Copy to clipboard",
                            lambda item=item: self.handleSelect(item))

            if item["blob_type"] == "text":
                iconUrls = self.icon_text
            elif item["blob_type"] == "image":
                iconUrls = self.icon_image
            elif item["blob_type"] == "file":
                iconUrls = self.icon_file
            else:
                iconUrls = self.icon_text

            result = StandardItem(id="%s" % item["id"],
                                  text=item["summary"],
                                  subtext=item["blob_type"],
                                  actions=[action],
                                  iconUrls=iconUrls)
            results.append(result)

        query.add(results)

    def handleSelect(self, item):
        debug(f"Selected {item}")
        try:
            id = "%s" % item["id"]
            subprocess.run([self.control_cmd, "get", "blob", id, "-d"])
        except subprocess.CalledProcessError as e:
            error("Csync write blob error: %s" % e)
            return
