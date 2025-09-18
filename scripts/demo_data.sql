-- Demo data for Terminalist screenshots
-- Run with: sqlite3 terminalist_debug.db < demo_data.sql

-- Clear existing data first
DELETE FROM task_labels;
DELETE FROM tasks;
DELETE FROM sections;
DELETE FROM projects;
DELETE FROM labels;

-- Insert Projects (work and personal mix)
INSERT INTO projects (id, name, color, is_favorite, is_inbox_project, order_index, parent_id) VALUES
('inbox', 'Inbox', 'grey', 0, 1, 0, NULL),
('work-main', 'Work Projects', 'blue', 0, 0, 1, NULL),
('personal', 'Personal', 'green', 0, 0, 2, NULL),
('shopping', 'Shopping & Errands', 'orange', 0, 0, 3, NULL),
('learning', 'Learning & Development', 'purple', 0, 0, 4, NULL),
('health', 'Health & Fitness', 'red', 0, 0, 5, NULL),
('home', 'Home Improvement', 'teal', 0, 0, 6, NULL),
-- Subprojects
('mobile-app', 'Mobile App Redesign', 'blue', 0, 0, 7, 'work-main'),
('website', 'Company Website', 'blue', 0, 0, 8, 'work-main'),
('vacation', 'Summer Vacation Planning', 'green', 0, 0, 9, 'personal');

-- Insert Labels
INSERT INTO labels (id, name, color, order_index, is_favorite) VALUES
('urgent', 'urgent', 'red', 1, 0),
('waiting', 'waiting-for', 'yellow', 2, 0),
('quick', 'quick-win', 'green', 3, 0);

-- Insert Sections
INSERT INTO sections (id, name, project_id, order_index) VALUES
('backend-section', 'Backend Development', 'mobile-app', 1),
('frontend-section', 'Frontend Development', 'mobile-app', 2),
('testing-section', 'Testing & QA', 'mobile-app', 3),
('content-section', 'Content & Copy', 'website', 1),
('design-section', 'Design & Assets', 'website', 2),
('groceries-section', 'Groceries', 'shopping', 1),
('errands-section', 'Errands', 'shopping', 2);

-- Insert Tasks with realistic content
INSERT INTO tasks (id, content, description, project_id, section_id, parent_id, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration) VALUES
-- Today's urgent tasks
('task001', 'Fix authentication bug in login flow', 'Users reporting 500 errors on login attempt', 'mobile-app', 'backend-section', NULL, 4, 1, date('now'), NULL, 0, NULL, NULL),
('task002', 'Review pull request #127 from Sarah', 'New user dashboard components', 'mobile-app', 'frontend-section', NULL, 3, 2, date('now'), NULL, 0, NULL, NULL),
('task003', 'Prepare slides for client presentation', 'Quarterly review meeting with Acme Corp', 'work-main', NULL, NULL, 4, 3, date('now'), datetime('now', '+3 hours'), 0, NULL, NULL),

-- Tomorrow's tasks
('task004', 'Update API documentation', 'Add new endpoints from v2.1 release', 'mobile-app', 'backend-section', NULL, 2, 4, date('now', '+1 day'), NULL, 0, NULL, NULL),
('task005', 'Schedule team standup meeting', 'Weekly sync for the mobile team', 'work-main', NULL, NULL, 2, 5, date('now', '+1 day'), NULL, 1, NULL, NULL),
('task006', 'Buy groceries for dinner party', NULL, 'shopping', 'groceries-section', NULL, 3, 6, date('now', '+1 day'), NULL, 0, NULL, NULL),

-- This week's tasks
('task007', 'Implement dark mode toggle', 'User-requested feature for accessibility', 'mobile-app', 'frontend-section', NULL, 2, 7, date('now', '+3 days'), NULL, 0, NULL, NULL),
('task008', 'Write unit tests for payment module', 'Increase test coverage to 85%', 'mobile-app', 'testing-section', NULL, 2, 8, date('now', '+4 days'), NULL, 0, NULL, NULL),
('task009', 'Research new hosting providers', 'Current costs are too high, need alternatives', 'website', NULL, NULL, 1, 9, date('now', '+5 days'), NULL, 0, NULL, NULL),

-- Personal tasks
('task010', 'Book flight tickets to Barcelona', 'Check prices on different airlines', 'vacation', NULL, NULL, 3, 10, date('now', '+2 days'), NULL, 0, NULL, NULL),
('task011', 'Find accommodation in Barcelona', 'Airbnb vs hotels comparison', 'vacation', NULL, NULL, 2, 11, date('now', '+3 days'), NULL, 0, NULL, NULL),
('task012', 'Morning workout routine', '30 min cardio + strength training', 'health', NULL, NULL, 2, 12, date('now'), datetime('now', '+16 hours'), 1, NULL, NULL),

-- Learning tasks
('task013', 'Complete Rust advanced patterns course', 'Chapter 7: Async programming', 'learning', NULL, NULL, 2, 13, date('now', '+7 days'), NULL, 0, NULL, NULL),
('task014', 'Read "Clean Architecture" chapter 12', 'Notes for team book club discussion', 'learning', NULL, NULL, 1, 14, date('now', '+10 days'), NULL, 0, NULL, NULL),

-- Home improvement
('task015', 'Fix leaky bathroom faucet', 'Need to buy new washer from hardware store', 'home', NULL, NULL, 3, 15, date('now', '+1 day'), NULL, 0, NULL, NULL),
('task016', 'Paint living room walls', 'Choose color and buy supplies', 'home', NULL, NULL, 1, 16, date('now', '+14 days'), NULL, 0, NULL, NULL),

-- Subtasks example
('task017', 'Plan mobile app architecture', 'Parent task for architecture planning', 'mobile-app', 'backend-section', NULL, 3, 17, date('now', '+2 days'), NULL, 0, NULL, NULL),
('task018', 'Design database schema', 'Tables for user profiles and preferences', 'mobile-app', 'backend-section', 'task017', 2, 18, date('now', '+1 day'), NULL, 0, NULL, NULL),
('task019', 'Create API endpoint specifications', 'REST API design for mobile client', 'mobile-app', 'backend-section', 'task017', 2, 19, date('now', '+2 days'), NULL, 0, NULL, NULL),
('task020', 'Set up CI/CD pipeline', 'Automated testing and deployment', 'mobile-app', 'backend-section', 'task017', 1, 20, date('now', '+3 days'), NULL, 0, NULL, NULL),

-- More realistic daily tasks
('task021', 'Respond to client emails', 'Follow up on project proposals', 'work-main', NULL, NULL, 2, 21, date('now'), NULL, 1, NULL, NULL),
('task022', 'Weekly grocery shopping', 'Milk, bread, vegetables, coffee', 'shopping', 'groceries-section', NULL, 1, 22, date('now', '+2 days'), NULL, 1, NULL, NULL),
('task023', 'Call mom', 'Check in and plan weekend visit', 'personal', NULL, NULL, 2, 23, date('now'), NULL, 0, NULL, NULL),
('task024', 'Update resume with recent projects', 'Add mobile app and website work', 'personal', NULL, NULL, 1, 24, date('now', '+7 days'), NULL, 0, NULL, NULL),

-- Inbox tasks (no project assignment)
('task025', 'Research team building activities', 'For next company retreat', 'inbox', NULL, NULL, 1, 25, NULL, NULL, 0, NULL, NULL),
('task026', 'Buy birthday gift for Alex', 'Their birthday is next month', 'inbox', NULL, NULL, 2, 26, date('now', '+20 days'), NULL, 0, NULL, NULL);

-- Insert task-label relationships
INSERT INTO task_labels (task_id, label_id) VALUES
('task001', 'urgent'),
('task003', 'urgent'),
('task006', 'quick'),
('task010', 'urgent'),
('task015', 'quick'),
('task021', 'quick'),
('task023', 'quick'),
('task005', 'waiting'),
('task009', 'waiting');