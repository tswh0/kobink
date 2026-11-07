class QCoreApplication:
    @staticmethod
    def translate(context, text, *arguments):
        return text

    @staticmethod
    def instance():
        return None


class QTranslator:
    def load(self, *arguments):
        return False


def qVersion():
    return "6.12.0"
