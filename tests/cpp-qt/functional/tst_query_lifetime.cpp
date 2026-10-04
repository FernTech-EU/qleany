#include "undo_redo/undo_redo_system.h"
#include <QCoreApplication>
#include <QCoro/QCoroTask>
#include <QPointer>
#include <QSemaphore>
#include <QSignalSpy>
#include <QTest>
#include <QThreadPool>

namespace UR = FullCppQtApp::Common::UndoRedo;

class TestQueryLifetime : public QObject
{
    Q_OBJECT
  private Q_SLOTS:
    void sharedQueryOutlivesHandler()
    {
        auto handler = std::make_unique<UR::QueryHandler>();
        auto query = handler->createQuery<int>(QStringLiteral("retained"));
        QPointer<UR::QueryBase> observer(query.get());
        QVERIFY(query->parent() == nullptr);
        handler.reset();
        QVERIFY(observer);
        query.reset();
        QVERIFY(observer.isNull());
    }

    void concurrentQueriesCompleteIndependently()
    {
        QThreadPool::globalInstance()->setMaxThreadCount(
            qMax(2, QThreadPool::globalInstance()->maxThreadCount()));
        UR::UndoRedoSystem system;
        QSignalSpy finished(system.queryHandler(), &UR::QueryHandler::queryFinished);
        auto gate = std::make_shared<QSemaphore>();
        // Always unblock the worker, including an early assertion failure.
        auto release = qScopeGuard([gate] { gate->release(); });
        auto first = system.createQuery<int>(QStringLiteral("first"));
        first->setQueryFunction(
            [gate]
            {
                gate->acquire();
                return 11;
            });
        auto second = system.createQuery<int>(QStringLiteral("second"));
        second->setQueryFunction([] { return 22; });
        auto firstCompleted = std::make_shared<bool>(false);
        auto firstTask = system.executeQueryAsync(first).then(
            [firstCompleted](int value)
            {
                *firstCompleted = true;
                return value;
            });
        QCOMPARE(QCoro::waitFor(system.executeQueryAsync(second)), 22);
        QVERIFY(!*firstCompleted);
        QCOMPARE(finished.count(), 1);
        QCOMPARE(qvariant_cast<std::shared_ptr<UR::QueryBase>>(finished.at(0).at(0)).get(), second.get());
        gate->release();
        QCOMPARE(QCoro::waitFor(firstTask), 11);
        QCOMPARE(finished.count(), 2);
        QCOMPARE(qvariant_cast<std::shared_ptr<UR::QueryBase>>(finished.at(1).at(0)).get(), first.get());
    }

    void pendingCoroutineOutlivesSystem()
    {
        auto system = std::make_unique<UR::UndoRedoSystem>();
        auto gate = std::make_shared<QSemaphore>();
        auto release = qScopeGuard([gate] { gate->release(); });
        auto query = system->createQuery<int>(QStringLiteral("pending"));
        query->setQueryFunction(
            [gate]
            {
                gate->acquire();
                return 42;
            });
        auto task = system->executeQueryAsync(query);
        system.reset();
        gate->release();
        QCOMPARE(QCoro::waitFor(task), 42);
    }

    void shutdownWithQueuedCompletion()
    {
        auto system = std::make_unique<UR::UndoRedoSystem>();
        auto query = system->createQuery<int>(QStringLiteral("queued"));
        query->setQueryFunction([] { return 7; });
        auto task = system->executeQueryAsync(query);
        // Finish worker work without delivering its completion event.
        QVERIFY(QThreadPool::globalInstance()->waitForDone(5000));
        system->shutdown();
        system.reset();
        QCOMPARE(QCoro::waitFor(task), 7);
    }
};
QTEST_GUILESS_MAIN(TestQueryLifetime)
#include "tst_query_lifetime.moc"
