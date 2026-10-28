import unittest

from summarize_station_validation import render_summary


class SummaryTests(unittest.TestCase):
    def test_summary_reports_health_without_urls_or_server_text(self):
        report = {
            "checked_at_unix": 1_793_222_307,
            "results": [
                {
                    "station_id": "skylab",
                    "status": "decoded",
                    "stage": "decoded_audio",
                    "attempts": 1,
                    "recovered": False,
                    "resolved_url": "https://example.invalid/token=secret",
                    "error": "private server diagnostics",
                },
                {
                    "station_id": "wcrb",
                    "status": "unavailable",
                    "stage": "endpoint",
                    "attempts": 2,
                    "recovered": False,
                },
                {
                    "station_id": "npr_pending",
                    "status": "unverified",
                    "stage": "not_checked",
                    "attempts": 0,
                    "recovered": False,
                },
            ],
        }

        summary = render_summary(report)

        self.assertIn("Decoded: **1/3**", summary)
        self.assertIn("| skylab | decoded | decoded_audio | 1 | no |", summary)
        self.assertIn("| wcrb | unavailable | endpoint | 2 | no |", summary)
        self.assertIn("| npr_pending | unverified | not_checked | 0 | no |", summary)
        self.assertNotIn("example.invalid", summary)
        self.assertNotIn("secret", summary)
        self.assertNotIn("private server diagnostics", summary)

    def test_untrusted_labels_cannot_inject_markdown(self):
        summary = render_summary(
            {
                "results": [
                    {
                        "station_id": "station | [injected]",
                        "status": "decoded | bad",
                        "stage": "decoded_audio | bad",
                        "attempts": 99_999,
                    }
                ]
            }
        )

        self.assertIn("| unknown | unknown | unknown | 255 | no |", summary)
        self.assertNotIn("injected", summary)

    def test_malformed_results_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "results array"):
            render_summary({"results": "not-an-array"})


if __name__ == "__main__":
    unittest.main()
