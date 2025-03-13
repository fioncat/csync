# -*- coding: utf-8 -*-

import json
import subprocess

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

    def defaultTrigger(self):
        return "cc "

    def handleTriggerQuery(self, query):
        if not query.isValid:
            return

        debug("Csync query: %s" % query.string)

        try:
            result = subprocess.run(
                ["csynctl", "get", "metadata", "-o", "json"],
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

            result = StandardItem(id="%s" % item["id"],
                                  text=item["summary"],
                                  subtext=item["blob_type"],
                                  actions=[action])
            results.append(result)

        query.add(results)

    def handleSelect(self, item):
        debug(f"Selected {item}")
